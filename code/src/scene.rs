use crate::objects::camera::Camera;
use crate::objects::light::LightSource;
use crate::objects::model3d::InteractiveModel;
use crate::objects::morph::Morph;

pub struct Scene {
    pub camera: Camera,
    pub light_source: LightSource,
    // pub object: Box<Morph>,
    pub object: Box<dyn InteractiveModel>,
}
