use crate::objects::camera::Camera;
use crate::objects::light::LightSource;
use crate::objects::model3d::InteractiveModel;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Scene {
    pub camera: Camera,
    pub light_source: LightSource,
    pub object: Option<Rc<RefCell<dyn InteractiveModel>>>,
}
