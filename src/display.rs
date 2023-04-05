use std::fmt;


struct DisplayFn<F>(F);

pub(crate) fn display_fn<'a, F>(body: F) -> impl fmt::Display + 'a
where
    F: Fn(&mut fmt::Formatter<'_>) -> fmt::Result + 'a,
{
    DisplayFn(body)
}

impl<F> fmt::Display for DisplayFn<F>
where
    F: Fn(&mut fmt::Formatter<'_>) -> fmt::Result,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0(f)
    }
}

pub fn count_digits(mut n: usize) -> usize {
    if n == 0 {
        1
    } else {
        let mut digits = 0;
        while n > 0 {
            digits += 1;
            n /= 10;
        }
        digits
    }
}