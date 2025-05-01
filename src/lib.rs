//! # BufBytes
//! ファイルから、バッファリングしながら、バイト(`u8`)ごとにデータを取り出すイテレーターです。  
//! unsafeを使ってるので、あんまり保証がないのが特徴です。  
//! ポインタ勉強用...  

use std::{io::{Error, Read, Result}, ptr::NonNull};

#[derive(Debug)]
pub struct BufBytes<B>
where
    B: Read,
{
    base: B,
    buf: Vec<u8>,
    buf_ptr: NonNull<u8>,
    buf_ptr_end: NonNull<u8>,
    error: Option<std::io::Error>,
}

impl<B> BufBytes<B>
where
    B: Read,
{
    /// BufBytesを作成
    /// 
    /// バッファーサイズは8192になります。
    pub fn new(base: B) -> Result<Self> {
        Self::with_capacity(base, 8192)
    }

    /// BufBytesを作成
    /// 
    /// バッファーサイズがいじれます。
    pub fn with_capacity(mut base: B, size: usize) -> Result<Self> {
        // バッファ作成し、base(ファイルなど)からデータを読み込む
        let mut buf = vec![0; size];
        // buf_lenは読み込めたデータ長=バッファのサイズ
        let buf_len = base.read(buf.as_mut())?;

        if buf_len == 0 {
            return Err(Error::other("0 size file"));
        }

        // バッファの先頭のポインタを取り出す。 これが、イテレーターのポインタともなる
        // イテレーターの終わりを判断するため、バッファ最後のポインタもとる
        let buf_ptr = NonNull::new(buf.as_mut_ptr()).unwrap();
        let buf_ptr_end = NonNull::new(&mut buf[buf_len-1] as *mut u8).unwrap();
        // let buf_ptr_end = unsafe { buf_ptr.as_ptr().add(buf_len) };
        Ok(Self {
            base,
            buf,
            buf_ptr,
            buf_ptr_end,
            // 途中baseからデータを読み込む際にエラーが起きた時は、
            // ここにエラーを入れる
            error: None,
        })
    }

    fn refill_buffer(&mut self) -> bool {
        // 再読み込みできたらtrueを返す
        match self.base.read(&mut self.buf) {
            Ok(0) => false,
            Ok(buf_len) => {
                // ポインタを再生成する
                self.buf_ptr = NonNull::new(self.buf.as_mut_ptr()).unwrap();
                self.buf_ptr_end = NonNull::new(&mut self.buf[buf_len-1] as *mut u8).unwrap();
                true
            },
            Err(e) => {
                self.error = Some(e);
                false
            },
        }
    }

    /// io操作中に生じたエラーを取得する
    pub fn get_err<'a>(&'a self) -> &'a Option<std::io::Error> {
        &self.error
    }

    /// io処理のエラーが発生したら、エラーを返す
    /// 
    /// クロージャ内でbytesイテレーターを操作し、正常に成功したらクロージャの戻り値が、  
    /// io処理中にエラーが発生していたら、エラーを返します。
    pub fn try_block<T>(&mut self, f: impl Fn(&mut Self)->T) -> std::result::Result<T, &std::io::Error> {
        let t = f(self);
        match self.get_err() {
            Some(err) => {
                Err(err)
            },
            None => Ok(t),
        }
    }
}

impl<B> Iterator for BufBytes<B>
where
    B: Read,
{
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.buf_ptr.as_ptr() > self.buf_ptr_end.as_ptr() {
            if !self.refill_buffer() {
                return None;
            }
        }
        unsafe {
            let res = self.buf_ptr.as_ref();
            self.buf_ptr = self.buf_ptr.add(1);
            Some(*res)
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::NamedTempFile;
    use std::io::{Seek, Write};

    use super::*;

    struct ErrorFile {
        error_bytes: usize,
        cursor: usize,
    }
    
    impl ErrorFile {
        fn new(error_bytes: usize) -> Self {
            Self{error_bytes, cursor:0}
        }
    }

    impl Read for ErrorFile {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
            self.cursor += buf.len();
            if self.cursor > self.error_bytes {
                return Err(Error::other("error"));
            }
            buf.fill(0);
            Ok(buf.len())
        }
    }

    // 8byte バッファーでデータを読み込む
    #[test]
    fn buf_8byte_test() {
        let base_txt = "abcdefg\nhijklmn\nopqrstu\nvwxyz00\n";

        // テストファイル作成
        let mut file = NamedTempFile::new().unwrap();
        file.write(base_txt.as_bytes()).unwrap();
        // 書き込み後、シークを0に戻す
        file.flush().unwrap();
        file.seek(std::io::SeekFrom::Start(0)).unwrap();

        let bytes = BufBytes::with_capacity(file, 8).unwrap();
        
        _ = bytes.zip(base_txt.bytes()).for_each(|(file, base)| {
            // println!("{}, {}", file, base);
            assert_eq!(file, base);
        });
    }

    // 0byteファイルを弾く
    #[test]
    fn zero_size_file_test() {
        let file = NamedTempFile::new().unwrap();
        let bytes = BufBytes::new(file);

        assert!(matches!(bytes, Err(_)));
    }

    // リード中にエラーが起きたときの動作
    #[test]
    fn read_error_test() {
        // 17byte目を読み込もうとするとエラーが返ってくる仮想ファイル
        let err_file = ErrorFile::new(17);
        let bytes = BufBytes::with_capacity(err_file, 8).unwrap();

        // nullにならなず、読み込めた範囲で帰ってくる
        assert_eq!(bytes.count(), 16);
    }

    // try block用テスト
    #[test]
    fn try_block_failed_test() {
        // 17byte目を読み込もうとするとエラーが返ってくる仮想ファイル
        let err_file = ErrorFile::new(17);
        let mut bytes = BufBytes::with_capacity(err_file, 8).unwrap();

        let res = bytes.try_block(|b| {
            b.count()
        });

        assert!(matches!(res, Err(_)));

    }

    #[test]
    fn try_block_success_test() {
        let base_txt = "abcdefg\nhijklmn\nopqrstu\nvwxyz00\n";

        // テストファイル作成
        let mut file = NamedTempFile::new().unwrap();
        file.write(base_txt.as_bytes()).unwrap();
        // 書き込み後、シークを0に戻す
        file.flush().unwrap();
        file.seek(std::io::SeekFrom::Start(0)).unwrap();

        let mut bytes = BufBytes::with_capacity(file, 8).unwrap();

        let res = bytes.try_block(|b| {
            b.count()
        });

        assert_eq!(32, res.unwrap())
    }

}