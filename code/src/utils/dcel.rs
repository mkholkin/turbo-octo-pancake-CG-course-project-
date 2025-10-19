use nalgebra::{Point3, Vector3};
use std::collections::HashMap;
use std::f64::consts::PI;

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
    pub fn new(vertices: Vec<Vertex>, connections: Vec<[usize; 2]>) -> Self {
        // let mut dcel = DCEL::default();
        // dcel.vertices = vertices;

        let mut half_edges = Vec::new();
        let mut edge_map: HashMap<usize, Vec<usize>> = HashMap::new();

        // 1. Создание полуребер.
        // Итерируем по всем каноническим сегментам и создаем пару близнецов.
        for &seg in connections.iter() {
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

            edge_map.entry(start).or_insert_with(Vec::new).push(he1_idx);
            edge_map.entry(end).or_insert_with(Vec::new).push(he2_idx);
        }

        // 2. Сортировка полуребер, исходящих из каждой вершины, по углу наклона.
        // Это ключевой шаг для правильного связывания граней.
        for (vertex_idx, outgoing_edges_indices) in edge_map.iter_mut() {
            let origin_point = &vertices[*vertex_idx];
            outgoing_edges_indices.sort_by(|a, b| {
                let a_dir = vertices[half_edges[half_edges[*a].twin].origin] - origin_point;
                let b_dir = vertices[half_edges[half_edges[*b].twin].origin] - origin_point;

                let ang_a = tangent_angle(&origin_point.coords, &a_dir);
                let ang_b = tangent_angle(&origin_point.coords, &b_dir);

                ang_a.partial_cmp(&ang_b).unwrap()
            })
        }

        // 3. Связывание указателей `next` и `prev`.
        // Создаем циклы, которые определяют границы граней.
        for (key, outgoing_edges_indices) in edge_map {
            if outgoing_edges_indices.len() < 2 {
                eprintln!(
                    "DEBUG: DCEL::new - недостаточно исходящих рёбер для вершины {:?}",
                    key
                );
                eprintln!("  количество рёбер: {}", outgoing_edges_indices.len());
                panic!(
                    "Вершина должна иметь минимум 2 исходящих ребра, получено: {}",
                    outgoing_edges_indices.len()
                );
            }

            let n = outgoing_edges_indices.len();
            for i in 0..n {
                let idx = outgoing_edges_indices[i];
                let twin_idx = half_edges[idx].twin;
                half_edges[twin_idx].next = Some(outgoing_edges_indices[(i + 1) % n]);
            }
        }

        // 4. Обход полуребер для нахождения граней.
        let mut faces = Vec::new();

        for i in 0..half_edges.len() {
            if half_edges[i].face.is_some() {
                continue;
            }

            let face = Face { edge: i };
            let face_idx = faces.len();
            faces.push(face);

            let mut curr_he_idx = i;
            loop {
                half_edges[curr_he_idx].face = Some(face_idx);
                curr_he_idx = half_edges[curr_he_idx].next.unwrap();
                if curr_he_idx == i {
                    break;
                }
            }
        }

        let dcel = DCEL {
            vertices,
            half_edges,
            faces,
        };

        dcel
    }

    pub fn get_face_vertices(&self, face_idx: usize) -> Vec<usize> {
        let mut vertices = Vec::new();

        let half_edge_idx = self.faces[face_idx].edge;
        let mut curr_he_idx = half_edge_idx;
        loop {
            let curr_he = &self.half_edges[curr_he_idx];

            vertices.push(curr_he.origin);
            curr_he_idx = curr_he.next.unwrap();
            if curr_he_idx == half_edge_idx {
                break;
            }
        }

        vertices
    }
}

/// Вычисляет угол направления в касательной плоскости вершины.
fn tangent_angle(normal: &Vector3<f64>, direction: &Vector3<f64>) -> f64 {
    // Шаг 1: Выбираем опорный вектор, не коллинеарный нормали.
    let mut ref_vector = if normal.x.abs() < 0.1 && normal.y.abs() < 0.1 {
        // Если нормаль близка к оси Z
        Vector3::new(1.0, 0.0, 0.0)
    } else {
        Vector3::new(0.0, 0.0, 1.0)
    };

    // Шаг 2: Строим ортонормированный базис (u, v) для касательной плоскости.
    let mut u = normal.cross(&ref_vector);

    // Если u нулевой, пробуем другой ref
    if u.norm_squared() < 1e-6 {
        ref_vector = if normal.x.abs() < 0.5 {
            Vector3::new(1.0, 0.0, 0.0)
        } else {
            Vector3::new(0.0, 1.0, 0.0)
        };
        u = normal.cross(&ref_vector);
    }
    u.normalize_mut();

    let v = normal.cross(&u); // Уже ортонормирован

    // Шаг 3: Проекция вектора направления на базис (u, v).
    let x = direction.dot(&u);
    let y = direction.dot(&v);

    // Шаг 4: Вычисляем угол в радианах.
    let angle = y.atan2(x);

    // Приводим угол к диапазону [0, 2π].
    if angle < 0.0 { angle + 2.0 * PI } else { angle }
}
