use self::component::Component;

mod component;

pub(crate) struct Cell {}

pub(crate) struct Grid {
    width: usize,
    height: usize,
    components: Vec<Box<dyn Component>>,
}

impl Grid {
    pub fn new(width: usize, height: usize) -> Grid {
        Grid {
            width,
            height,
            components: Vec::new(),
        }
    }

    pub fn add_component(&mut self, comp: impl Component + 'static) {
        self.components.push(Box::new(comp));
    }

    pub fn draw(&mut self) -> Vec<Vec<String>> {
        todo!()
    }
}
