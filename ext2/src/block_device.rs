pub trait BlockDevice: Send + Sync + 'static {
    const BLOCK_SIZE: usize;
    fn read_block(&self, block_id: usize, buf: &mut [u8]);
    fn write_block(&self, block_id: usize, buf: &[u8]);
}
