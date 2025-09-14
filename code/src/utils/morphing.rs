use crate::objects::model3d::{Model3D, Triangle};
use crate::objects::triangle_mesh::TriangleMesh;
use crate::utils::dcel::{DCEL, Vertex};
use crate::utils::triangles::barycentric;
use delaunator::{Point, triangulate};
use nalgebra::{Point3, Vector3, Vector4};
use std::collections::{HashMap, HashSet};

const EPS: f32 = 1e-6;

type Segment = [usize; 2];

/// Вычисляет площадь треугольника, заданного тремя вершинами.
fn triangle_area(v1: &Point3<f32>, v2: &Point3<f32>, v3: &Point3<f32>) -> f32 {
    let cross_product = (v2 - v1).cross(&(v3 - v1));
    0.5 * cross_product.norm()
}

/// Вычисляет центр масс полигональной сетки.
pub fn center_of_mass(mesh: &TriangleMesh) -> Vector3<f32> {
    // TODO: работа с вершинами в координатах мира (после трансформаций)

    let mut total_area = 0.0;
    let mut weighted_center = Vector3::zeros();

    for tri in mesh.triangles() {
        let v1 = &mesh.vertices()[tri.0];
        let v2 = &mesh.vertices()[tri.1];
        let v3 = &mesh.vertices()[tri.2];

        let area = triangle_area(v1, v2, v3);
        let center = (v1.coords + v2.coords + v3.coords) / 3.0;

        total_area += area;
        weighted_center += center * area;
    }

    if total_area > 0.0 {
        weighted_center / total_area
    } else {
        Vector3::zeros()
    }
}

pub fn parametrize_mesh(mesh: &mut TriangleMesh) {
    let center = center_of_mass(mesh);

    for v in mesh.vertices_mut() {
        *v = Point3::from((v.coords - center).normalize());
    }
}

/// Checks if a point `p` is on the arc between points `start` and `end`.
/// All points are expected to be on the unit sphere.
fn is_on_arc(p: &Point3<f32>, start: &Point3<f32>, end: &Point3<f32>) -> bool {
    let p_vec = p.coords;
    let start_vec = start.coords;
    let end_vec = end.coords;

    // Check if the point p is on the great circle defined by start and end.
    let cross_product = start_vec.cross(&end_vec);
    let scalar_triple_product = cross_product.dot(&p_vec);
    if scalar_triple_product.abs() > EPS {
        return false;
    }

    // A correct angular check: the angle from start to p plus the angle
    // from p to end must equal the angle from start to end.
    // We can use the dot products for this.
    let total_angle = start_vec.dot(&end_vec);
    let angle_sp = start_vec.dot(&p_vec);
    let angle_pe = p_vec.dot(&end_vec);

    // This check ensures p is between start and end.
    if total_angle <= angle_sp && total_angle <= angle_pe {
        return true;
    }

    false
}

/// Finds the intersection point of two great-circle arcs on a unit sphere.
/// Returns `Some(Point3<f32>)` if a unique intersection is found, otherwise `None`.
fn intersect_arcs(arc_1: [&Point3<f32>; 2], arc_2: [&Point3<f32>; 2]) -> Option<Point3<f32>> {
    // 1. Calculate the normals of the great circles.
    // The normal is the cross product of the arc's endpoints.
    let normal_1 = arc_1[0].coords.cross(&arc_1[1].coords);
    let normal_2 = arc_2[0].coords.cross(&arc_2[1].coords);

    // 2. Find the intersection line of the two great circles.
    // This line is orthogonal to both normal vectors.
    let intersection_line = normal_1.cross(&normal_2);

    // 3. Handle edge cases where great circles are parallel or the same.
    if intersection_line.norm_squared() < EPS * EPS {
        return None; // No unique intersection point.
    }

    // 4. Normalize the intersection line to get a point on the unit sphere.
    //Unit<Vector>
    let p1 = Point3::from(intersection_line.normalize());
    let p2 = -p1;

    // 5. Check if either of the two intersection points lies on both arcs.
    if is_on_arc(&p1, arc_1[0], arc_1[1]) && is_on_arc(&p1, arc_2[0], arc_2[1]) {
        return Some(p1);
    }

    if is_on_arc(&p2, arc_1[0], arc_1[1]) && is_on_arc(&p2, arc_2[0], arc_2[1]) {
        return Some(p2);
    }

    // No intersection found on the arcs.
    None
}

fn get_mesh_segments(mesh: &TriangleMesh) -> impl Iterator<Item = Segment> {
    mesh.triangles()
        .iter()
        .flat_map(|&tri| {
            let mut s1 = [tri.0, tri.1];
            s1.sort_unstable();

            let mut s2 = [tri.1, tri.2];
            s2.sort_unstable();

            let mut s3 = [tri.2, tri.0];
            s3.sort_unstable();

            [s1, s2, s3]
        })
        .into_iter()
}

fn find_or_add_vertex(vertices: &mut Vec<Point3<f32>>, point: Point3<f32>) -> usize {
    for (i, v) in vertices.iter().enumerate() {
        if (v.coords - point.coords).norm_squared() < EPS {
            return i;
        }
    }
    let new_index = vertices.len();
    vertices.push(point);
    new_index
}
/// Основная функция для построения DCEL из пересечения двух сеток.
/// Примечание: Это заглушка для `build_dcel`, так как полная реализация очень сложна.
/// Функция сосредоточена на определении уникальных сегментов и вершин.
pub fn create_dcel_map(mesh_a: &TriangleMesh, mesh_b: &TriangleMesh) -> DCEL {
    // Объединяем все вершины из обеих сеток в один изменяемый список.
    let mut all_vertices = mesh_a.vertices().clone();
    all_vertices.extend(mesh_b.vertices().clone());

    // Карта для хранения всех вершин, которые лежат на каждом отрезке.
    // Ключ — это канонический отрезок ([usize; 2]), а значение — это набор индексов вершин.
    let mut segment_map: HashMap<Segment, HashSet<usize>> = HashMap::new();

    // todo: сделать хешсеты из сегментов
    let segments_a: Vec<Segment> = get_mesh_segments(&mesh_a).collect();
    // Корректируем индексы, чтобы они соответствовали объединенному списку вершин.
    let segments_b: Vec<Segment> = get_mesh_segments(&mesh_b)
        .map(|mut s| {
            let offset = mesh_a.vertices().len();
            s[0] += offset;
            s[1] += offset;
            s
        })
        .collect();

    // Add all segments from mesh_a
    for s in get_mesh_segments(&mesh_a) {
        segment_map.entry(s).or_insert_with(HashSet::new);
    }

    // Add all segments from mesh_b with offset
    for mut s in get_mesh_segments(&mesh_b) {
        let offset = mesh_a.vertices().len();
        s[0] += offset;
        s[1] += offset;
        segment_map.entry(s).or_insert_with(HashSet::new);
    }

    // Находим точки пересечения между всеми дугами на единичной сфере.
    for seg_a in segments_a {
        for &seg_b in &segments_b {
            let arc_1 = [&all_vertices[seg_a[0]], &all_vertices[seg_a[1]]];
            let arc_2 = [&all_vertices[seg_b[0]], &all_vertices[seg_b[1]]];

            // Если найдено пересечение, добавляем новую вершину и отмечаем ее на обоих сегментах.
            if let Some(intersection_point) = intersect_arcs(arc_1, arc_2) {
                let inter_idx = find_or_add_vertex(&mut all_vertices, intersection_point);
                segment_map.get_mut(&seg_a).unwrap().insert(inter_idx);
                segment_map.get_mut(&seg_b).unwrap().insert(inter_idx);
            }
        }
    }

    // Генерируем финальный список подотрезков на основе точек пересечения.
    // Подотрезки образуются в результате разбиения исходных отрезков точками пересечения сеток.
    let mut all_segments: HashSet<Segment> = HashSet::new();

    // Итерируем по карте исходных сегментов и новых точек, которые на них лежат.
    for ([start_idx, end_idx], points_set) in segment_map.into_iter() {
        let mut points: Vec<usize> = points_set.into_iter().collect();

        // Сортируем точки вдоль дуги на основе их удаления относительно начальной точки.
        let start_coords = all_vertices[start_idx].coords;

        points.sort_unstable_by(|&a_idx, &b_idx| {
            let a_coords = all_vertices[a_idx].coords;
            let b_coords = all_vertices[b_idx].coords;

            let dist_a = (start_coords - a_coords).norm_squared();
            let dist_b = (start_coords - b_coords).norm_squared();
            dist_a.partial_cmp(&dist_b).unwrap()
        });

        points.insert(0, start_idx);
        points.push(end_idx);
        points.dedup();

        // Создаем новые подсегменты из отсортированного списка точек.
        for i in 0..points.len() - 1 {
            let mut seg = [points[i], points[i + 1]];
            if seg[0] == seg[1] {
                assert!(false);
                continue;
            }
            // Делаем сегмент каноническим, сортируя индексы.
            if seg[0] > seg[1] {
                seg.swap(0, 1);
            }

            all_segments.insert(seg);
        }
    }

    // 7. Это заглушка для финального шага.
    // `all_vertices` и `all_segments` теперь содержат всю необходимую информацию
    // для построения полностью соединенной DCEL. Функция `build_dcel` (здесь не реализована)
    // будет принимать эти две коллекции и строить финальную структуру DCEL, связывая
    // вершины, полуребра и грани.
    // TODO: возможно передавать слайсом или как-то еще чтобы не копировать лишний раз

    // println!("{:?}\n", all_segments);

    println!("Создание DCEL::new");
    println!("vertices = np.array({:?})", all_vertices);
    println!("connections = np.array({:?})", all_segments);
    DCEL::new(all_vertices, all_segments.into_iter().collect())
}

/// Триангулирует плоскую грань многогранника с использованием триангуляции Делоне.
fn triangulate_face(face_vertices: &Vec<&Vertex>, v_idx: &Vec<usize>) -> Vec<usize> {
    assert!(face_vertices.len() >= 3);

    // 1. Находим нормаль к грани многогранника
    // Поскольку грань может содержать отрезки, лежащие на одной прямой,
    // подбираем вектор, не параллельный первому
    let v1 = face_vertices[1] - face_vertices[0];
    let mut normal = Vector3::default();
    for i in 2..face_vertices.len() {
        normal = v1.cross(&(face_vertices[i] - face_vertices[0]));
        if normal.norm_squared() > EPS * EPS {
            break;
        }
    }

    if !(normal.norm_squared() > EPS) {
        println!("{:?},", v_idx);
    }

    // FIXME: Possible bug here
    assert!(normal.norm_squared() > EPS * EPS);
    normal.normalize_mut();

    // 2. Создаем ортонормированный базис грани [u, v].
    // Выбираем произвольный вектор и проецируем его на плоскость грани.
    let mut random_vec = Vector3::new(1., 0., 0.);
    if random_vec.dot(&normal) > 0.99 {
        // Если почти параллелен нормали, выбираем другой
        random_vec = Vector3::new(0., 1., 0.);
    }

    // Проецируем вектор на плоскость грани
    let mut u_vec = random_vec - random_vec.dot(&normal) * normal;
    assert!(u_vec.norm_squared() > EPS);
    u_vec.normalize_mut();

    // Второй вектор базиса
    let v_vec = normal.cross(&u_vec).normalize();

    // 3. Проецируем 3D-вершины на 2D-плоскость, используя новый базис
    // Находим центр масс грани
    let face_center: Vector3<f32> = face_vertices.iter().map(|p| p.coords).sum();
    let face_center = face_center / face_vertices.len() as f32;

    // Проецируем точки
    let mut projected_points_2d = Vec::new();
    // TODO: возможно неправильное проецирование
    for v in face_vertices {
        let vec_from_center = v.coords - face_center;
        let point = Point {
            x: vec_from_center.dot(&u_vec).into(),
            y: vec_from_center.dot(&v_vec).into(),
        };

        projected_points_2d.push(point);
    }

    // 4. Триангулируем грань при помощи триангуляции Делоне
    let triangulation = triangulate(projected_points_2d.as_slice());

    triangulation.triangles
}

pub fn triangulate_dcel(dcel: &DCEL) -> Vec<Triangle> {
    let mut triangles = Vec::new();

    for face_idx in 0..dcel.faces.len() {
        let vertex_indices = dcel.get_face_vertices(face_idx);
        let face_vertices_refs: Vec<&Vertex> =
            vertex_indices.iter().map(|&i| &dcel.vertices[i]).collect();
        let local_triangles = triangulate_face(&face_vertices_refs, &vertex_indices);
        let global_triangles: Vec<Triangle> = local_triangles
            .chunks(3)
            .map(|chunk| {
                (
                    vertex_indices[chunk[0]],
                    vertex_indices[chunk[1]],
                    vertex_indices[chunk[2]],
                )
            })
            .collect();

        triangles.extend(global_triangles);
    }

    triangles
}

// Найти треугольник на сетке, которому принадлежит точка.
// Возвращает индекс треугольника и барицентрические координаты точки в этом треугольнике.
fn find_enclosing_triangle(p: &Vertex, mesh: &TriangleMesh) -> (usize, Vector3<f32>) {
    let mesh_vertices = mesh.vertices();

    for (i, tri) in mesh.triangles().iter().enumerate() {
        let v0 = &mesh_vertices[tri.0];
        let v1 = &mesh_vertices[tri.1];
        let v2 = &mesh_vertices[tri.2];

        // 1. Находим нормаль к плоскости треугольника, направленную от центра сферы
        let mut normal = (v1 - v0).cross(&(v2 - v0)).normalize();

        // Разворачиваем нормаль, если она направленна в центр
        if normal.dot(&v0.coords) < 0. {
            normal = -normal;
        }

        // 2. Проецируем точку на плоскость треугольника
        let t = normal.dot(&p.coords);

        // Отбрасываем треугольники на противоположной стороне сферы
        if t < 0. {
            continue;
        }

        let projected_point = p * (normal.dot(&v0.coords) / t);

        // 3. Определяем принадлежность точки треугольнику по барицентрическим координатам
        let bary = barycentric(&projected_point, &v0, &v1, &v2);

        if bary.iter().all(|&coord| coord > -EPS) {
            return (i, bary);
        }
    }

    panic!("No triangle found. Impossible");
}

// Расположить рассчитать реальные координаты точке на сетке объекта
pub fn relocate_vertices_on_mesh(vertices: &Vec<Vertex>, mesh: &TriangleMesh) -> Vec<Vertex> {
    let mut relocated_vertices = Vec::new();

    for v in vertices {
        let (tri_idx, bary) = find_enclosing_triangle(&v, mesh);

        let tri = mesh.triangles()[tri_idx];
        relocated_vertices.push(Vertex::from(
            bary.x * mesh.vertices()[tri.0].coords
                + bary.y * mesh.vertices()[tri.1].coords
                + bary.z * mesh.vertices()[tri.2].coords,
        ));
    }

    relocated_vertices
}
