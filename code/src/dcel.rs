// struct Face<T> {
//     edge: Option<&HalfEdge<T>>,
// }
// 
// struct HalfEdge<T> {
//     origin: Option<&T>,     // Начальная вершина
//     twin: Option<&Self>,    // Парное полуребро
//     face: Option<&Face<T>>, // Принадлежащая грань
//     next: Option<&Self>,    // Следующее полуребро грани
// }
// 
// impl<T> HalfEdge<T> {
//     pub fn new() -> Self {
//         Self {
//             origin: None,
//             twin: None,
//             face: None,
//             next: None,
//         }
//     }
// }
// 
// struct DCEL<T> {
//     vertices: Vec<T>,
//     
//     faces: Vec<Face<T>>,
// }
