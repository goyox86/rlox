use rlox_parser::scanner::Scanner;

pub struct Compiler<'source> {
    source: &'source str,
}

impl<'source> Compiler<'source> {
    pub fn new(source: &'source str) -> Self {
        Self { source }
    }

    pub fn compile(&self) {
        let mut line: usize = 0;

        let mut scanner = Scanner::new(self.source);

        loop {
            let token = scanner.scan_token();

            if token.line != line {
                println!("{:0>4} ", token.line);
            } else {
                println!("    |  ")
            }

            println!("{:?}", token);

            if token.is_eof() {
                break;
            }
        }
    }
}
