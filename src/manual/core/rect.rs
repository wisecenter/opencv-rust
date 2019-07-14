use std::{
    fmt,
    ops::{Add, AddAssign, Mul, Sub, SubAssign},
};

use num::{NumCast, ToPrimitive, Zero};

use crate::core::{Point_, RotatedRect, Size_, ValidPointType, ValidSizeType};

valid_types!(ValidRectType, i32, f32, f64);

#[inline(always)]
fn partial_min<T: PartialOrd>(a: T, b: T) -> T {
    if a <= b { a } else { b }
}

#[inline(always)]
fn partial_max<T: PartialOrd>(a: T, b: T) -> T {
    if b >= a { b } else { a }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
/// [docs.opencv.org](https://docs.opencv.org/master/d2/d44/classcv_1_1Rect__.html)
pub struct Rect_<T: ValidRectType> {
    pub x: T,
    pub y: T,
    pub width: T,
    pub height: T,
}

impl<T: ValidRectType> Rect_<T> {
    #[inline]
    pub fn new(x: T, y: T, width: T, height: T) -> Self {
        Self { x, y, width, height }
    }

    #[inline]
    pub fn from_point_size(pt: Point_<T>, sz: Size_<T>) -> Self where T: ValidPointType + ValidSizeType {
        Self::new(pt.x, pt.y, sz.width, sz.height)
    }

    #[inline]
    pub fn from_points(pt1: Point_<T>, pt2: Point_<T>) -> Self where T: ValidPointType + PartialOrd + Sub<Output=T> {
        let x = partial_min(pt1.x, pt2.x);
        let y = partial_min(pt1.y, pt2.y);
        Self::new(x, y, partial_max(pt1.x, pt2.x) - x, partial_max(pt1.y, pt2.y) - y)
    }

    #[inline]
    pub fn tl(&self) -> Point_<T> where T: ValidPointType {
        Point_::new(self.x, self.y)
    }

    #[inline]
    pub fn br(&self) -> Point_<T> where T: ValidPointType + Add<Output=T> {
        Point_::new(self.x + self.width, self.y + self.height)
    }

    #[inline]
    pub fn size(&self) -> Size_<T> where T: ValidSizeType {
        Size_::new(self.width, self.height)
    }

    #[inline]
    pub fn area(&self) -> T where T: Mul<Output=T> {
        self.width * self.height
    }

    #[inline]
    pub fn empty(&self) -> bool where T: Zero + PartialOrd {
        self.width <= T::zero() || self.height <= T::zero()
    }

    #[inline]
    pub fn contains(&self, pt: Point_<T>) -> bool where T: ValidPointType + Add<Output=T> + PartialOrd {
        self.x <= pt.x && pt.x < self.x + self.width && self.y <= pt.y && pt.y < self.y + self.height
    }

    #[inline]
    pub fn to<D: ValidRectType + NumCast>(&self) -> Option<Rect_<D>> where T: ToPrimitive {
        Some(Rect_ { x: D::from(self.x)?, y: D::from(self.y)?, width: D::from(self.width)?, height: D::from(self.height)? })
    }
}

impl<T: ValidRectType + Default> Default for Rect_<T> {
    fn default() -> Self {
        Self { x: Default::default(), y: Default::default(), width: Default::default(), height: Default::default() }
    }
}

impl<P, R> Add<Point_<P>> for Rect_<R>
    where
        P: ValidPointType,
        R: ValidRectType + AddAssign<P>
{
    type Output = Rect_<R>;

    fn add(mut self, rhs: Point_<P>) -> Self::Output {
        self += rhs;
        self
    }
}

impl<P, R> Sub<Point_<P>> for Rect_<R>
    where
        P: ValidPointType,
        R: ValidRectType + SubAssign<P>
{
    type Output = Rect_<R>;

    fn sub(mut self, rhs: Point_<P>) -> Self::Output {
        self -= rhs;
        self
    }
}

impl<S, R> Add<Size_<S>> for Rect_<R>
    where
        S: ValidSizeType,
        R: ValidRectType + AddAssign<S>
{
    type Output = Rect_<R>;

    fn add(mut self, rhs: Size_<S>) -> Self::Output {
        self += rhs;
        self
    }
}

impl<S, R> Sub<Size_<S>> for Rect_<R>
    where
        S: ValidSizeType,
        R: ValidRectType + SubAssign<S>
{
    type Output = Rect_<R>;

    fn sub(mut self, rhs: Size_<S>) -> Self::Output {
        self -= rhs;
        self
    }
}

impl<P, R> AddAssign<Point_<P>> for Rect_<R>
    where
        P: ValidPointType,
        R: ValidRectType + AddAssign<P>
{
    fn add_assign(&mut self, rhs: Point_<P>) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl<P, R> SubAssign<Point_<P>> for Rect_<R>
    where
        P: ValidPointType,
        R: ValidRectType + SubAssign<P>
{
    fn sub_assign(&mut self, rhs: Point_<P>) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl<S, R> AddAssign<Size_<S>> for Rect_<R>
    where
        S: ValidSizeType,
        R: ValidRectType + AddAssign<S>
{
    fn add_assign(&mut self, rhs: Size_<S>) {
        self.width += rhs.width;
        self.height += rhs.height;
    }
}

impl<S, R> SubAssign<Size_<S>> for Rect_<R>
    where
        S: ValidSizeType,
        R: ValidRectType + SubAssign<S>
{
    fn sub_assign(&mut self, rhs: Size_<S>) {
        self.width -= rhs.width;
        self.height -= rhs.height;
    }
}

impl fmt::Debug for RotatedRect {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("RotatedRect")
            .field("angle", &self.angle().map_err(|_| fmt::Error)?)
            .field("center", &self.center().map_err(|_| fmt::Error)?)
            .field("size", &self.size().map_err(|_| fmt::Error)?)
            .finish()
    }
}


#[test]
fn test_partial() {
    assert_eq!(1., partial_min(1., 2.));
    assert_eq!(1., partial_min(2., 1.));
    assert_eq!(1., partial_min(1., 1.));
    assert_eq!(1, partial_min(1, 2));
    assert_eq!(1, partial_min(2, 1));
    assert_eq!(1, partial_min(1, 1));

    assert_eq!(2., partial_max(1., 2.));
    assert_eq!(2., partial_max(2., 1.));
    assert_eq!(2., partial_max(2., 2.));
    assert_eq!(2, partial_max(1, 2));
    assert_eq!(2, partial_max(2, 1));
    assert_eq!(2, partial_max(2, 2));
}