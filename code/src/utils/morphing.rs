use crate::config::RELAXATION_ROUNDS_LIMIT;
use crate::objects::model3d::{Model3D, Triangle};
use crate::objects::triangle_mesh::TriangleMesh;
use crate::utils::dcel::{DCEL, Vertex};
use crate::utils::triangles::barycentric;
use delaunator::{Point, triangulate};
use itertools::izip;
use nalgebra::{Matrix4, Point3, Vector3, Vector4};
use std::collections::{HashMap, HashSet};
use std::error::Error;

const EPS: f64 = 1e-6;

type Segment = [usize; 2];

/// Вычисляет площадь треугольника, заданного тремя вершинами.
fn triangle_area(v1: &Point3<f64>, v2: &Point3<f64>, v3: &Point3<f64>) -> f64 {
    let cross_product = (v2 - v1).cross(&(v3 - v1));
    0.5 * cross_product.norm()
}

/// Вычисляет центр масс полигональной сетки.
pub fn center_of_mass(mesh: &TriangleMesh) -> Vector3<f64> {
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

fn collect_neighbors(mesh: &TriangleMesh) -> Vec<HashSet<usize>> {
    let mut neighbors = vec![HashSet::new(); mesh.vertices().len()];

    for tri in mesh.triangles() {
        neighbors[tri.0].extend(&[tri.1, tri.2]);
        neighbors[tri.1].extend(&[tri.0, tri.2]);
        neighbors[tri.2].extend(&[tri.0, tri.1]);
    }

    neighbors
}

fn get_orientations(vertices: &Vec<Vertex>, triangles: &Vec<Triangle>) -> Vec<f64> {
    triangles
        .iter()
        .map(|tri| {
            let v0 = vertices[tri.0];
            let v1 = vertices[tri.1];
            let v2 = vertices[tri.2];

            v0.coords.cross(&(v1.coords)).dot(&(v2.coords)).signum()
        })
        .collect()
}

fn relax_mesh(parametrized_mesh: &mut TriangleMesh, original_orientations: &Vec<f64>) {
    let epsilon_threshold = 1e-3;

    let neighbors = collect_neighbors(parametrized_mesh);

    // Релаксация сетки
    let mut orientations = get_orientations(
        parametrized_mesh.vertices_world(),
        parametrized_mesh.triangles(),
    );
    let mut orientations_established = original_orientations.iter().eq(orientations.iter());
    let mut epsilon_reached = true;
    let mut round_no: usize = 0;

    while (!(orientations_established && epsilon_reached)) && round_no < RELAXATION_ROUNDS_LIMIT {
        // 1. Сохраняем положение вершин перед релаксацией
        let prev_vertices = parametrized_mesh.vertices_world().clone();

        // 2. Выполняем один раунд релаксации
        {
            let vertices = parametrized_mesh.vertices_world_mut();

            for i in 0..vertices.len() {
                let new_pos = neighbors[i]
                    .iter()
                    .map(|neighbor_idx| prev_vertices[*neighbor_idx].coords)
                    .sum::<Vector3<f64>>()
                    .normalize();
                vertices[i] = Vertex::from(new_pos);
            }

            // Достигнут эпсилон-порог (вершины почти не сдвинулись)
            epsilon_reached = prev_vertices
                .iter()
                .zip(vertices.iter())
                .all(|(prev, curr)| (prev - curr).norm() < epsilon_threshold);

            // Центрирование сферы для избежания коллапса вершин
            let mean: Vector3<f64> =
                vertices.iter().map(|v| v.coords).sum::<Vector3<f64>>() / vertices.len() as f64;
            vertices.iter_mut().for_each(|v| *v -= mean);
        }

        // Главное условие остановки: Ориентации граней совпадают с оригинальными (нет вывернутых граней)
        orientations = get_orientations(
            parametrized_mesh.vertices_world(),
            parametrized_mesh.triangles(),
        );
        orientations_established = original_orientations.iter().eq(orientations.iter());

        round_no += 1;
    }

    println!("{}", round_no);
}

pub fn parametrize_mesh(mesh: &mut TriangleMesh) {
    let vertices_world = mesh.vertices_world();
    let original_orientations = izip!(mesh.triangles(), mesh.normals())
        .map(|(tri, normal)| {
            let origin = vertices_world[tri.0].coords - normal.xyz();
            let v0 = vertices_world[tri.0].coords - origin;
            let v1 = vertices_world[tri.1].coords - origin;
            let v2 = vertices_world[tri.2].coords - origin;
            v0.cross(&v1).dot(&v2).signum()
        })
        .collect();

    // TODO: нужно искать не центр масс, а внутреннюю точку
    let center = center_of_mass(mesh);
    for v in mesh.vertices_world_mut() {
        *v = Point3::from((v.coords - center).normalize());
    }

    relax_mesh(mesh, &original_orientations);

    mesh.vertices = mesh.vertices_world().clone();
    mesh.model_matrix = Matrix4::identity();
}

/// Checks if a point `p` is on the arc between points `start` and `end`.
/// All points are expected to be on the unit sphere.
fn is_on_arc(p: &Point3<f64>, start: &Point3<f64>, end: &Point3<f64>) -> bool {
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
/// Returns `Some(Point3<f64>)` if a unique intersection is found, otherwise `None`.
fn intersect_arcs(arc_1: [&Point3<f64>; 2], arc_2: [&Point3<f64>; 2]) -> Option<Point3<f64>> {
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

fn get_mesh_segments(mesh: &TriangleMesh) -> HashSet<Segment> {
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
        .collect()
}

fn find_or_add_vertex(vertices: &mut Vec<Point3<f64>>, point: Point3<f64>) -> usize {
    for (i, v) in vertices.iter().enumerate() {
        if (v.coords - point.coords).norm() < EPS {
            return i;
        }
    }
    let new_index = vertices.len();
    vertices.push(point);
    new_index
}

/// Проверяет, лежит ли точка на дуге между двумя точками на единичной сфере
fn point_lies_on_arc(point: &Point3<f64>, start: &Point3<f64>, end: &Point3<f64>) -> bool {
    let p_vec = point.coords;
    let start_vec = start.coords;
    let end_vec = end.coords;

    // Проверяем, лежит ли точка на большом круге, определяемом start и end
    let cross_product = start_vec.cross(&end_vec);
    if cross_product.norm() < EPS {
        // start и end совпадают или противоположны
        return (p_vec - start_vec).norm() < EPS || (p_vec - end_vec).norm() < EPS;
    }

    let scalar_triple_product = cross_product.dot(&p_vec);
    if scalar_triple_product.abs() > EPS {
        return false;
    }

    // Проверяем, что точка лежит между start и end на дуге
    let angle_start_end = start_vec.dot(&end_vec).clamp(-1.0, 1.0).acos();
    let angle_start_point = start_vec.dot(&p_vec).clamp(-1.0, 1.0).acos();
    let angle_point_end = p_vec.dot(&end_vec).clamp(-1.0, 1.0).acos();

    (angle_start_point + angle_point_end - angle_start_end).abs() < EPS
}

/// Создает карту уникальных вершин, объединяя совпадающие точки из двух сеток
fn create_unified_vertex_map(
    mesh_a: &TriangleMesh,
    mesh_b: &TriangleMesh,
) -> (Vec<Point3<f64>>, Vec<usize>, Vec<usize>) {
    let mut unified_vertices = Vec::new();
    let mut mapping_a = Vec::new();
    let mut mapping_b = Vec::new();

    // Добавляем вершины из первой сетки
    for vertex in mesh_a.vertices_world() {
        let idx = find_or_add_vertex(&mut unified_vertices, *vertex);
        mapping_a.push(idx);
    }

    // Добавляем вершины из второй сетки, проверяя на дубликаты
    for vertex in mesh_b.vertices_world() {
        let idx = find_or_add_vertex(&mut unified_vertices, *vertex);
        mapping_b.push(idx);
    }

    (unified_vertices, mapping_a, mapping_b)
}

/// Находит все вершины одной сетки, которые лежат на рёбрах другой сетки
fn find_vertices_on_edges(
    vertex_mapping: &[usize],
    segments: &[Segment],
    all_vertices: &[Point3<f64>],
    segment_map: &mut HashMap<Segment, HashSet<usize>>,
) {
    for &vertex_idx in vertex_mapping {
        let vertex = &all_vertices[vertex_idx];

        for &segment in segments {
            let start = &all_vertices[segment[0]];
            let end = &all_vertices[segment[1]];

            // Проверяем, лежит ли вершина на этом сегменте
            if point_lies_on_arc(vertex, start, end)
                && vertex_idx != segment[0]
                && vertex_idx != segment[1]
            {
                segment_map
                    .entry(segment)
                    .or_insert_with(HashSet::new)
                    .insert(vertex_idx);
            }
        }
    }
}
/// Основная функция для построения DCEL из пересечения двух сеток.
/// Корректно обрабатывает совпадающие вершины и случаи, когда вершина лежит на ребре.
pub fn create_dcel_map(mesh_a: &TriangleMesh, mesh_b: &TriangleMesh) -> DCEL {
    // 1. Создаем унифицированную карту вершин, избегая дублирования
    let (mut all_vertices, mapping_a, mapping_b) = create_unified_vertex_map(mesh_a, mesh_b);

    // 2. Получаем сегменты из обеих сеток с правильными индексами
    let segments_a: Vec<Segment> = get_mesh_segments(mesh_a)
        .into_iter()
        .map(|s| [mapping_a[s[0]], mapping_a[s[1]]])
        .map(|mut s| {
            s.sort_unstable();
            s
        })
        .collect();

    let segments_b: Vec<Segment> = get_mesh_segments(mesh_b)
        .into_iter()
        .map(|s| [mapping_b[s[0]], mapping_b[s[1]]])
        .map(|mut s| {
            s.sort_unstable();
            s
        })
        .collect();

    // 3. Карта для хранения всех вершин, которые лежат на каждом отрезке
    let mut segment_map: HashMap<Segment, HashSet<usize>> = HashMap::new();

    // Добавляем все отрезки в карту
    for &s in &segments_a {
        segment_map.entry(s).or_insert_with(HashSet::new);
    }
    for &s in &segments_b {
        segment_map.entry(s).or_insert_with(HashSet::new);
    }

    // 4. Находим вершины, которые лежат на рёбрах другой сетки
    // Проверяем вершины сетки A на рёбрах сетки B
    find_vertices_on_edges(&mapping_a, &segments_b, &all_vertices, &mut segment_map);

    // Проверяем вершины сетки B на рёбрах сетки A
    find_vertices_on_edges(&mapping_b, &segments_a, &all_vertices, &mut segment_map);

    // 5. Находим точки пересечения между дугами
    for &seg_a in &segments_a {
        for &seg_b in &segments_b {
            // Пропускаем, если сегменты имеют общие вершины
            if seg_a[0] == seg_b[0]
                || seg_a[0] == seg_b[1]
                || seg_a[1] == seg_b[0]
                || seg_a[1] == seg_b[1]
            {
                continue;
            }

            let arc_1 = [&all_vertices[seg_a[0]], &all_vertices[seg_a[1]]];
            let arc_2 = [&all_vertices[seg_b[0]], &all_vertices[seg_b[1]]];

            if let Some(intersection_point) = intersect_arcs(arc_1, arc_2) {
                let inter_idx = find_or_add_vertex(&mut all_vertices, intersection_point);
                segment_map.get_mut(&seg_a).unwrap().insert(inter_idx);
                segment_map.get_mut(&seg_b).unwrap().insert(inter_idx);
            }
        }
    }

    // 6. Генерируем финальный список подотрезков
    let mut all_segments: HashSet<Segment> = HashSet::new();

    for ([start_idx, end_idx], points_set) in segment_map.into_iter() {
        let mut points: Vec<usize> = points_set.into_iter().collect();

        // Сортируем точки вдоль дуги на основе их расстояния от начальной точки
        let start_coords = all_vertices[start_idx].coords;

        points.sort_unstable_by(|&a_idx, &b_idx| {
            let a_coords = all_vertices[a_idx].coords;
            let b_coords = all_vertices[b_idx].coords;

            let dist_a = (start_coords - a_coords).norm_squared();
            let dist_b = (start_coords - b_coords).norm_squared();
            dist_a.partial_cmp(&dist_b).unwrap()
        });

        // Добавляем начальную и конечную вершины
        points.insert(0, start_idx);
        points.push(end_idx);
        points.dedup();

        // Создаем новые подсегменты
        for i in 0..points.len() - 1 {
            let mut seg = [points[i], points[i + 1]];
            if seg[0] == seg[1] {
                continue; // Пропускаем вырожденные сегменты
            }

            // Делаем сегмент каноническим
            if seg[0] > seg[1] {
                seg.swap(0, 1);
            }

            all_segments.insert(seg);
        }
    }

    // TODO: возможно передавать слайсом или как-то еще чтобы не копировать лишний раз
    DCEL::new(all_vertices, all_segments.into_iter().collect())
}

/// Триангулирует плоскую грань многогранника с использованием триангуляции Делоне.
fn triangulate_face(face_vertices: &Vec<&Vertex>) -> Result<Vec<usize>, Box<dyn Error>> {
    // Проверка минимального количества вершин
    if face_vertices.len() < 3 {
        eprintln!(
            "DEBUG: triangulate_face - недостаточно вершин: {} (требуется >= 3)",
            face_vertices.len()
        );
        return Err(format!(
            "Грань должна содержать минимум 3 вершины, получено: {}",
            face_vertices.len()
        )
        .into());
    }

    // 1. Находим нормаль к грани многогранника
    // Поскольку грань может содержать отрезки, лежащие на одной прямой,
    // подбираем вектор, не параллельный первому
    let v1 = face_vertices[1] - face_vertices[0];
    let mut normal = Vector3::default();
    for i in 2..face_vertices.len() {
        normal = v1.cross(&(face_vertices[i] - face_vertices[0]));
        if normal.norm() > 0. {
            break;
        }
    }

    // Проверка валидности нормали
    if normal.norm() < f64::EPSILON {
        eprintln!("DEBUG: triangulate_face - не удалось вычислить нормаль к грани");
        eprintln!("  v1 = {}", v1);
        if face_vertices.len() > 2 {
            eprintln!("  v2 = {}", face_vertices[2] - face_vertices[0]);
        }
        eprintln!("  normal.norm() = {}", normal.norm());
        eprintln!("  количество вершин: {}", face_vertices.len());
        return Err("Не удалось вычислить нормаль к грани: все вершины коллинеарны".into());
    }

    normal.normalize_mut();

    // 2. Создаем ортонормированный базис грани [u, v].
    // Используем первое ребро грани как один из базисных векторов
    let mut u_vec = face_vertices[1] - face_vertices[0];

    if u_vec.norm_squared() < f64::EPSILON {
        eprintln!("DEBUG: triangulate_face - вырожденное первое ребро грани");
        eprintln!("  u_vec.norm_squared() = {}", u_vec.norm_squared());
        return Err("Не удалось построить базис: первое ребро вырождено".into());
    }

    u_vec.normalize_mut();

    // Второй вектор базиса получаем как векторное произведение нормали и первого базисного вектора
    let mut v_vec = normal.cross(&u_vec);

    // Проверяем валидность второго базисного вектора
    if v_vec.norm_squared() < f64::EPSILON {
        eprintln!("DEBUG: triangulate_face - вырожденный второй базисный вектор");
        eprintln!("  v_vec.norm_squared() = {}", v_vec.norm_squared());
        eprintln!("  normal = {}", normal);
        eprintln!("  u_vec = {}", u_vec);
        return Err("Не удалось построить ортонормированный базис грани".into());
    }

    v_vec.normalize_mut();

    // 3. Проецируем 3D-вершины на 2D-плоскость, используя новый базис
    // Используем первую вершину как начало координат
    let origin = face_vertices[0].coords;

    // Проецируем точки
    let mut projected_points_2d = Vec::new();
    for v in face_vertices {
        let vec_from_origin = v.coords - origin;
        let point = Point {
            x: vec_from_origin.dot(&u_vec).into(),
            y: vec_from_origin.dot(&v_vec).into(),
        };

        projected_points_2d.push(point);
    }

    // 4. Триангулируем грань при помощи триангуляции Делоне
    let triangulation = triangulate(projected_points_2d.as_slice());

    Ok(triangulation.triangles)
}

pub fn triangulate_dcel(dcel: &DCEL) -> Result<Vec<Triangle>, Box<dyn Error>> {
    let mut triangles = Vec::new();

    for face_idx in 0..dcel.faces.len() {
        let vertex_indices = dcel.get_face_vertices(face_idx);
        let face_vertices_refs: Vec<&Vertex> =
            vertex_indices.iter().map(|&i| &dcel.vertices[i]).collect();

        let local_triangles = triangulate_face(&face_vertices_refs).map_err(|e| {
            eprintln!(
                "DEBUG: triangulate_dcel - ошибка триангуляции грани {}: {}",
                face_idx, e
            );
            format!("Ошибка триангуляции грани {}: {}", face_idx, e)
        })?;

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

    Ok(triangles)
}

// Найти треугольник на сетке, которому принадлежит точка.
// Возвращает индекс треугольника и барицентрические координаты точки в этом треугольнике.
fn find_enclosing_triangle(p: &Vertex, mesh: &TriangleMesh) -> (usize, Vector3<f64>) {
    let mesh_vertices = mesh.vertices_world();

    for (i, tri) in mesh.triangles().iter().enumerate() {
        let v0 = &mesh_vertices[tri.0];
        let v1 = &mesh_vertices[tri.1];
        let v2 = &mesh_vertices[tri.2];

        // 1. Находим нормаль к плоскости треугольника, направленную от центра сферы
        let mut normal = (v1 - v0).cross(&(v2 - v1)).normalize();

        // Разворачиваем нормаль, если она направленна в центр
        if normal.dot(&v0.coords) < 0.0 {
            normal = -normal;
        }

        // 2. Проецируем точку на плоскость треугольника
        let t = normal.dot(&p.coords);

        // Отбрасываем треугольники на противоположной стороне сферы
        if t < 0. {
            continue;
        }

        let projected_point = if t < f64::EPSILON {
            *p
        } else {
            p * (normal.dot(&v0.coords) / t)
        };

        if t < f64::EPSILON {
            println!("{}", projected_point);
        }

        // 3. Определяем принадлежность точки треугольнику по барицентрическим координатам
        let bary = barycentric(&projected_point, &v0, &v1, &v2);

        if bary.iter().all(|&coord| coord > -1e-12) {
            return (i, bary);
        }
    }

    panic!("No triangle found. This should not happen for a closed mesh on a sphere.");
}

// Расположить рассчитать реальные координаты точке на сетке объекта
pub fn relocate_vertices_on_mesh(
    parametrized_vertices: &Vec<Vertex>,
    parametrized_mesh: &TriangleMesh,
    real_vertices: &Vec<Vertex>,
) -> Vec<Vertex> {
    let mut relocated_vertices = Vec::new();

    for v in parametrized_vertices {
        let (tri_idx, bary) = find_enclosing_triangle(&v, parametrized_mesh);
        let tri = parametrized_mesh.triangles()[tri_idx];

        relocated_vertices.push(Vertex::from(
            bary.x * real_vertices[tri.0].coords
                + bary.y * real_vertices[tri.1].coords
                + bary.z * real_vertices[tri.2].coords,
        ));
    }

    relocated_vertices
}

pub fn find_normals(
    parametrized_vertices: &Vec<Vertex>,
    triangles: &Vec<Triangle>,
    parametrized_mesh: &TriangleMesh,
) -> Vec<Vector4<f64>> {
    let mut normals = Vec::new();

    for tri in triangles {
        let center = Point3::from(
            (parametrized_vertices[tri.0].coords
                + parametrized_vertices[tri.1].coords
                + parametrized_vertices[tri.2].coords)
                / 3.,
        );

        let (tri_idx, _) = find_enclosing_triangle(&center, parametrized_mesh);
        normals.push(parametrized_mesh.normals()[tri_idx]);
    }

    normals
}
