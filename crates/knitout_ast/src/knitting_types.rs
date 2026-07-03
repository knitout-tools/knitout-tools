#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Carrier {
    id: u32,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Direction {
    Left,
    Right,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Bed {
    Front,
    Back,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum IsSlider {
    Slider,
    NotSlider,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Needle {
    bed: Bed,
    is_slider: IsSlider,
    index: i32,
}
