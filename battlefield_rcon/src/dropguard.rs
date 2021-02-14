
pub struct DropGuard<T> {
    label: String,
    t: T,
}

impl <T> DropGuard<T> {
    pub fn new(t: T, label: impl Into<String>) -> Self {
        let label = label.into();
        println!("Created {}!", label);
        Self { t , label }
    }

    pub fn get(&self) -> &T {
        &self.t
    }

    pub fn get_mut(&mut self) -> &mut T {
        // println!("{}.get_mut()", self.label);
        &mut self.t
    }

    pub fn label(&self) -> &String {
        &self.label
    }
}

impl <T> Drop for DropGuard<T> {
    fn drop(&mut self) {
        println!("Dropped {}!", self.label);
    }
}