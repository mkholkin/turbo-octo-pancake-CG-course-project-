import os
import json
from collections import defaultdict
import argparse
import matplotlib.pyplot as plt
import re
import csv
import matplotlib

matplotlib.rcParams['hatch.linewidth'] = 0.1

def collect_raw_data(criterion_root: str):
    """
    Собирает сырые данные о времени выполнения для каждой группы и ее этапов.

    Returns:
        Словарь вида: {'имя_группы': {'имя_этапа': время_нс, ...}, ...}
    """
    all_groups_data = {}

    if not os.path.isdir(criterion_root):
        print(f"Ошибка: Директория '{criterion_root}' не найдена. Запустите 'cargo bench' сначала.")
        return None

    group_dirs = [d for d in os.listdir(criterion_root) if os.path.isdir(os.path.join(criterion_root, d))]

    for group_name in group_dirs:
        group_path = os.path.join(criterion_root, group_name)
        stage_totals = defaultdict(float)

        for root, _, files in os.walk(group_path):
            if os.path.basename(root) == 'base' and 'benchmark.json' in files:
                try:
                    parent_dir_name = os.path.basename(os.path.dirname(root))
                    with open(os.path.join(root, 'estimates.json'), 'r') as f:
                        point_estimate = json.load(f).get("mean", {}).get("point_estimate")

                    if point_estimate is not None:
                        stage_totals[parent_dir_name] += point_estimate
                except Exception as e:
                    print(f"Предупреждение: Не удалось обработать '{root}'. Ошибка: {e}")

        if stage_totals:
            all_groups_data[group_name] = dict(stage_totals)

    if not all_groups_data:
        print("Ошибка: Не найдено валидных данных для анализа.")
        return None

    print(f"Найдено и обработано {len(all_groups_data)} групп бенчмарков.")
    return all_groups_data


def generate_reports(all_data: dict, output_dir: str):
    """
    Генерирует два CSV-файла и данные для итоговой диаграммы.
    """
    os.makedirs(output_dir, exist_ok=True)

    # --- Подготовка к CSV №1: Сырые тайминги ---
    all_stage_names = sorted(list(set(stage for stages in all_data.values() for stage in stages.keys())))

    csv1_path = os.path.join(output_dir, 'timings_raw.csv')
    with open(csv1_path, 'w', newline='', encoding='utf-8') as f:
        writer = csv.writer(f)
        # Формируем заголовок
        header = ['pair', 'total_time_ns'] + [f'{stage}_time_ns' for stage in all_stage_names]
        writer.writerow(header)

        # Заполняем строки
        for group_name, stages in all_data.items():
            total_time = sum(stages.values())
            row = [group_name, round(total_time, 2)] + [round(stages.get(s, 0), 2) for s in all_stage_names]
            writer.writerow(row)

    print(f"Отчет с сырыми таймингами сохранен в '{csv1_path}'")

    # --- Подготовка к CSV №2: Анализ процентов ---
    stage_percentages = defaultdict(list)
    for group_name, stages in all_data.items():
        total_time = sum(stages.values())
        if total_time > 0:
            for stage_name, stage_time in stages.items():
                percentage = (stage_time / total_time) * 100
                stage_percentages[stage_name].append(percentage)

    csv2_path = os.path.join(output_dir, 'percentage_summary.csv')
    summary_for_chart = []
    with open(csv2_path, 'w', newline='', encoding='utf-8') as f:
        writer = csv.writer(f)
        writer.writerow(['stage', 'min_percentage', 'max_percentage', 'avg_percentage'])

        for stage_name in all_stage_names:
            percentages = stage_percentages.get(stage_name, [0])
            min_p = min(percentages)
            max_p = max(percentages)
            avg_p = sum(percentages) / len(percentages)
            writer.writerow([stage_name, f"{min_p:.2f}", f"{max_p:.2f}", f"{avg_p:.2f}"])
            summary_for_chart.append((stage_name, avg_p))

    print(f"Сводный отчет по процентам сохранен в '{csv2_path}'")

    return summary_for_chart




def create_summary_pie_chart(summary_data: list, output_dir: str):
    """
    Создает одну итоговую круговую диаграмму на основе средних процентов.
    """
    labels = [item[0] for item in summary_data]
    sizes = [item[1] for item in summary_data]
    cmap = plt.get_cmap("tab20c")
    colors = [cmap(i) for i in range(len(sizes))]

    fig, ax = plt.subplots(figsize=(12, 8), subplot_kw=dict(aspect="equal"))

    wedges, texts, autotexts = ax.pie(
        sizes, labels=None, autopct='%1.1f%%', startangle=140, pctdistance=0.85,
        hatch=["..", "++", "oo", "xx"], colors=colors,
        wedgeprops={'linewidth': 2, 'edgecolor': 'white'}
    )

    plt.setp(autotexts, size=10, weight="bold")
    for autotext in autotexts:
        autotext.set_bbox(dict(facecolor='white', alpha=0.6, edgecolor='none', pad=1))

    ax.set_title("Средний вклад этапов в общее время морфинга", size=16, weight="bold")

    ax.legend(wedges, labels,
              title="Этапы",
              loc="upper left",
              bbox_to_anchor=(1, 0, 0.5, 1))

    plt.tight_layout()
    chart_filename = os.path.join(output_dir, 'summary_pie_chart.pdf')
    plt.savefig(chart_filename, bbox_inches='tight', format="pdf")
    print(f"Итоговая диаграмма сохранена в '{chart_filename}'")


def main():
    parser = argparse.ArgumentParser(description="Анализ результатов Criterion.rs и генерация отчетов.")
    parser.add_argument(
        "--root", default="target/criterion",
        help="Корневая директория с результатами Criterion (по умолчанию: 'target/criterion')"
    )
    parser.add_argument(
        "--output", default="analysis_results",
        help="Директория для сохранения отчетов (по умолчанию: 'analysis_results')"
    )
    args = parser.parse_args()

    # 1. Сбор сырых данных
    raw_data = collect_raw_data(args.root)
    if not raw_data:
        return

    # 2. Генерация CSV-отчетов и получение данных для графика
    summary_data = generate_reports(raw_data, args.output)

    # 3. Создание итоговой диаграммы
    if summary_data:
        create_summary_pie_chart(summary_data, args.output)


if __name__ == "__main__":
    main()