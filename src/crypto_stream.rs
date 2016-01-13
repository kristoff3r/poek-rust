trait CryptoStream {
    fn init<F>(callback: F) -> Self
        where F : FnMut(&[u8]);

    fn encrypt<F>(&mut self, data: &[u8], callback: F)
        where F : FnMut(&[u8]);

    fn decrypt<F>(&mut self, data: &[u8], callback: F)
        where F : FnMut(&[u8]);
}
