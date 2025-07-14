use crate::platform::PlatformManager;
use crate::utils::Result;
use std::cell::RefCell;
use std::rc::Rc;

pub struct MockPlatformManager {
    pub close_calls: Rc<RefCell<Vec<(String, String)>>>,
}

impl Default for MockPlatformManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MockPlatformManager {
    pub fn new() -> Self {
        MockPlatformManager {
            close_calls: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

impl PlatformManager for MockPlatformManager {
    fn close_ide_window(&self, session_id: &str, ide_name: &str, _state_dir: &str) -> Result<()> {
        self.close_calls
            .borrow_mut()
            .push((session_id.to_string(), ide_name.to_string()));
        Ok(())
    }
}
