use std::time::Instant;
use zeroize::zeroize;
pub struct Session {
    pub seed: Vec<u8>,
    pub unlocked_at: Instant,
    pub timeout: u64,
}


pub impl Session{
    pub fn new(seed: Vec<u8>) {
        Session {
            seed,
            unlocked_at: Instant::now(),
            timeout: 300
        }
    }
    
    pub fn is_expired(&self) -> bool {
        self.unlocked_at.elapsed().as_secs() > self.timeout_secs
    }


    pub fn refresh(&mut self) {
        self.unlocked_at = Instant::now();
    }

    pub fn clear(&mut self) {
        self.seed.zeroize();
        self.seed.clear();
    }
}
