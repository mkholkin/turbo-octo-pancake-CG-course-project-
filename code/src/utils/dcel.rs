use nalgebra::{Point3, Vector3};
use std::collections::BTreeMap;

pub type Vertex = Point3<f64>;

#[derive(Default)]
pub struct Face {
    edge: usize,
}

#[derive(Default)]
pub struct HalfEdge {
    origin: usize,       // Начальная вершина
    twin: usize,         // Парное полуребро
    face: Option<usize>, // Принадлежащая грань
    next: Option<usize>, // Следующее полуребро грани
}

#[derive(Default)]
pub struct DCEL {
    pub vertices: Vec<Vertex>,
    pub half_edges: Vec<HalfEdge>,
    pub faces: Vec<Face>,
}

impl DCEL {
    pub fn new(
        vertices: Vec<Vertex>,
        connections: impl IntoIterator<Item = [usize; 2]>,
    ) -> Result<Self, String> {
        let mut half_edges = Vec::new();
        let mut edge_map: BTreeMap<usize, Vec<usize>> = BTreeMap::new();

        // 1. Создание полуребер.
        // Итерируем по всем каноническим сегментам и создаем пару близнецов.
        for seg in connections {
            let (start, end) = (seg[0], seg[1]);
            let mut he1 = HalfEdge::default();
            let mut he2 = HalfEdge::default();

            let he1_idx = half_edges.len();
            let he2_idx = he1_idx + 1;

            he1.origin = start;
            he1.twin = he2_idx;

            he2.origin = end;
            he2.twin = he1_idx;

            half_edges.push(he1);
            half_edges.push(he2);

            edge_map.entry(start).or_default().push(he1_idx);
            edge_map.entry(end).or_default().push(he2_idx);
        }

        // 2. Сортировка полуребер, исходящих из каждой вершины, по углу наклона.
        // Это ключевой шаг для правильного связывания граней.
        for (vertex_idx, outgoing_edges_indices) in edge_map.iter_mut() {
            if outgoing_edges_indices.len() < 2 {
                return Err(format!(
                    "Вершина {} должна иметь минимум 2 исходящих ребра, получено: {}",
                    vertex_idx,
                    outgoing_edges_indices.len()
                ));
            }

            let origin_point = &vertices[*vertex_idx];
            Self::sort_edges_by_angle(origin_point, outgoing_edges_indices, &half_edges, &vertices);
        }

        // 3. Связывание указателей `next` и `prev`.
        // Создаем циклы, которые определяют границы граней.
        for outgoing_edges_indices in edge_map.values() {
            // Используем &edge_map, чтобы не перемещать значения
            let n = outgoing_edges_indices.len();
            for i in 0..n {
                let h_curr_idx = outgoing_edges_indices[i];
                let h_prev_idx = outgoing_edges_indices[(i + n - 1) % n];

                let t_prev_idx = half_edges[h_prev_idx].twin;
                half_edges[t_prev_idx].next = Some(h_curr_idx);
            }
        }

        // 4. Обход полуребер для нахождения граней.
        let mut faces = Vec::new();
        for he_idx in 0..half_edges.len() {
            if half_edges[he_idx].face.is_none() {
                // Нашли новое, еще не назначенное полуребро -> нашли новую грань.
                let face_idx = faces.len();
                faces.push(Face { edge: he_idx });

                // Обходим цикл `next`, чтобы пометить все полуребра этой грани.
                let mut curr_he_idx = he_idx;
                loop {
                    let he = &mut half_edges[curr_he_idx];
                    if he.face.is_some() {
                        break;
                    }
                    he.face = Some(face_idx);
                    curr_he_idx = he.next.ok_or("HalfEdge loop is broken")?;

                    if curr_he_idx == he_idx {
                        break;
                    }
                }
            }
        }

        let dcel = DCEL {
            vertices,
            half_edges,
            faces,
        };

        Ok(dcel)
    }

    pub fn get_face_vertices(&self, face_idx: usize) -> Vec<usize> {
        let mut vertices = Vec::new();
        let start_he_idx = self.faces[face_idx].edge;
        let mut curr_he_idx = start_he_idx;

        loop {
            let curr_he = &self.half_edges[curr_he_idx];
            vertices.push(curr_he.origin);
            curr_he_idx = curr_he.next.unwrap();

            // Прерываем цикл, когда возвращаемся к ИСХОДНОМУ РЕБРУ.
            if curr_he_idx == start_he_idx {
                break;
            }
        }

        vertices
    }

    fn sort_edges_by_angle(
        origin_point: &Vertex,
        outgoing_edges_indices: &mut [usize],
        half_edges: &[HalfEdge],
        vertices: &[Vertex],
    ) {
        let normal = origin_point.coords.normalize(); // Всегда лучше нормализовать

        // 1. Создаем НАДЕЖНЫЙ ортонормированный базис в касательной плоскости.
        // Выбираем глобальную ось, которая не коллинеарна нормали.
        let reference_vec = if normal.z.abs() < 0.999 {
            // Если мы не у полюсов
            Vector3::new(0.0, 0.0, 1.0) // Используем ось Z
        } else {
            // Если мы у полюсов, используем ось X
            Vector3::new(1.0, 0.0, 0.0)
        };

        // Проецируем опорный вектор на касательную плоскость, чтобы получить `u`
        let u = (reference_vec - normal * reference_vec.dot(&normal)).normalize();
        // `v` перпендикулярен `normal` и `u`
        let v = normal.cross(&u).normalize();

        // 2. Сортируем ребра, используя этот надежный базис.
        outgoing_edges_indices.sort_by(|&a_idx, &b_idx| {
            // Находим конечную точку для каждого полуребра
            let p_a = vertices[half_edges[half_edges[a_idx].twin].origin];
            let p_b = vertices[half_edges[half_edges[b_idx].twin].origin];

            let a_dir = (p_a - origin_point).normalize();
            let b_dir = (p_b - origin_point).normalize();

            let angle_a = tangent_angle(&u, &v, &a_dir);
            let angle_b = tangent_angle(&u, &v, &b_dir);

            angle_a.total_cmp(&angle_b)
        });
    }
}

/// Вычисляет угол направления в касательной плоскости вершины.
fn tangent_angle(u: &Vector3<f64>, v: &Vector3<f64>, direction: &Vector3<f64>) -> f64 {
    let x = direction.dot(u);
    let y = direction.dot(v);
    y.atan2(x)
}
