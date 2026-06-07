pub trait HardwareBridge {
    fn connect() -> Result<Self, String> where Self: Sized;
    fn send_apdu(&self, command: &[u8]) -> Result<Vec<u8>, String>;
}
