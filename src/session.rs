pub struct Session {
    pub seed: Vec<u8>,
    pub unlocked_at: Instant,
    pub created_at: Instant,
    pub timeout: u64,
    pub max_lifetime: u64,
}

impl Session {
    pub fn new(seed: Vec<u8>) -> Self {
        Session {
            seed,
            unlocked_at: Instant::now(),
            created_at: Instant::now(),
            timeout: 300,
            max_lifetime: 3600,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.unlocked_at.elapsed().as_secs() > self.timeout
            || self.created_at.elapsed().as_secs() > self.max_lifetime
    }

    pub fn refresh(&mut self) -> bool {
        if self.created_at.elapsed().as_secs() > self.max_lifetime {
            self.clear();
            return false;
        }
        self.unlocked_at = Instant::now();
        true
    }

    pub fn clear(&mut self) {
        self.seed.zeroize();
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.seed.zeroize();
    }
}