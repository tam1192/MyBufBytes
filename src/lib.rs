//! # BufBytes
//! ファイルから、バッファリングしながら、バイト(`u8`)ごとにデータを取り出すイテレーターです。  
//! unsafeを使ってるので、あんまり保証がないのが特徴です。  
//! ポインタ勉強用...  

use std::{io::{Read, Result}, ptr::NonNull};

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

    // pub fn try_block<T>(&mut self, f: impl Fn(&mut Self)->T) -> std::result::Result<T, &std::io::Error> {
    //     let t = f(self);
    //     match self.get_err() {
    //         Some(err) => {
    //             Err(err)
    //         },
    //         None => Ok(t),
    //     }
    // }
}

impl<B> Iterator for BufBytes<B>
where
    B: Read,
{
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.buf_ptr.as_ptr() == self.buf_ptr_end.as_ptr() {
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
