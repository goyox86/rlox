use std::fmt::Debug;
use std::fmt::Display;

#[derive(Debug)]
pub struct Stack<T>(Vec<T>);

impl<T> Stack<T> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        self.0.pop()
    }

    #[inline]
    pub fn push(&mut self, value: T) {
        self.0.push(value)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    pub fn peek(&self, distance: usize) -> Option<&T> {
        if self.is_empty() || distance > self.0.len() - 1 {
            return None;
        }

        self.0.get(self.0.len() - 1 - distance)
    }

    pub fn reset(&mut self) {
        self.0.clear()
    }
}

impl<T: Debug + Display> Display for Stack<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[ ")?;
        for elem in &self.0 {
            write!(f, "{} ", elem)?;
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peek() {
        let mut stack: Stack<i32> = Stack::new();
        stack.push(1);
        stack.push(2);
        stack.push(3);

        assert_eq!(Some(&3), stack.peek(0));
        assert_eq!(Some(&2), stack.peek(1));
        assert_eq!(Some(&1), stack.peek(2));
    }

    #[test]
    fn test_peek_empty() {
        let stack: Stack<i32> = Stack::new();

        assert_eq!(None, stack.peek(0));
        assert_eq!(None, stack.peek(1));
    }

    #[test]
    fn test_peek_pushing_popping() {
        let mut stack: Stack<i32> = Stack::new();
        stack.push(1);
        assert_eq!(Some(&1), stack.peek(0));
        stack.push(2);
        assert_eq!(Some(&1), stack.peek(1));
        stack.push(3);
        assert_eq!(Some(&1), stack.peek(2));
        let _ = stack.pop();
        assert_eq!(None, stack.peek(2));
        assert_eq!(Some(&1), stack.peek(1));
    }
}
