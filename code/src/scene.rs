use crate::objects::camera::Camera;
use crate::objects::light::LightSource;
use crate::objects::triangle_mesh::TriangleMesh;

struct Scene {
    camera: Camera,
    light_source: LightSource,
    object: TriangleMesh
}