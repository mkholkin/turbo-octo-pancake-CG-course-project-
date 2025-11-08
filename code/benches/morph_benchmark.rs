use code::objects::model3d::Model3D;
use code::objects::triangle_mesh::TriangleMesh;
use code::utils::morphing::{
    create_supermesh, find_normals, parametrize_mesh, relocate_vertices_on_mesh,
};
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use std::path::Path; // Импортируем Path

const INPUTS: &[(&str, &str)] = &[
    ("models/apple2.obj", "models/pear.obj"),
    ("models/apple2.obj", "models/lemon.obj"),
    ("models/apple2.obj", "models/banana.obj"),
    ("models/lemon.obj", "models/banana.obj"),
    ("models/lemon.obj", "models/pear.obj"),
    ("models/banana.obj", "models/pear.obj"),
];

fn morph_stages_benchmark(c: &mut Criterion) {
    // Теперь цикл будет создавать свою собственную группу для каждой пары
    for (source_path, target_path) in INPUTS {
        // --- Получаем короткие имена для названия группы ---
        let source_name = Path::new(source_path)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap();
        let target_name = Path::new(target_path)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap();

        // --- Создаем УНИКАЛЬНУЮ группу для этой пары моделей ---
        let group_name = format!("Morph: {} to {}", source_name, target_name);
        let mut group = c.benchmark_group(&group_name);

        // --- Подготовка данных ---
        let source = TriangleMesh::from_obj(source_path).expect("Failed to load source");
        let target = TriangleMesh::from_obj(target_path).expect("Failed to load target");

        // --- Этап 1: Параметризация ---
        // Используем iter_with_setup для более точных измерений
        group.bench_function("Параметризация", |b| {
            b.iter_with_setup(
                || (source.clone(), target.clone()),
                |(mut s, mut t)| {
                    parametrize_mesh(black_box(&mut s));
                    parametrize_mesh(black_box(&mut t));
                },
            )
        });

        // "По-настоящему" выполняем шаги для передачи данных дальше
        let mut parametrized_source_mesh = source.clone();
        parametrize_mesh(&mut parametrized_source_mesh);
        let mut parametrized_target_mesh = target.clone();
        parametrize_mesh(&mut parametrized_target_mesh);

        // --- Этап 2: Построение суперсетки ---
        group.bench_function("Построение суперсетки", |b| {
            b.iter(|| black_box(create_supermesh(&parametrized_source_mesh, &parametrized_target_mesh).unwrap()))
        });

        let (vertices, triangles) =
            create_supermesh(&parametrized_source_mesh, &parametrized_target_mesh).unwrap();

        // --- Этап 3: Перенос вершин ---
        group.bench_function("Перенос вершин", |b| {
            b.iter(|| {
                black_box(relocate_vertices_on_mesh(&vertices, &parametrized_source_mesh, source.vertices_world()).unwrap());
                black_box(relocate_vertices_on_mesh(&vertices, &parametrized_target_mesh, target.vertices_world()).unwrap());
            })
        });

        // --- Этап 4: Перенос нормалей ---
        group.bench_function("Перенос нормалей", |b| {
            b.iter(|| {
                black_box(find_normals(&vertices, &triangles, &parametrized_source_mesh, source.normals()).unwrap());
                black_box(find_normals(&vertices, &triangles, &parametrized_target_mesh, target.normals()).unwrap());
            })
        });

        group.finish();
    }
}

criterion_group!(benches, morph_stages_benchmark);
criterion_main!(benches);