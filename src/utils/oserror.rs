#[derive(Debug)]
pub struct OSError {
    message: &'static str,
}

impl OSError {
    pub fn new(message: &'static str) -> Self {
        Self { message }
    }
}
