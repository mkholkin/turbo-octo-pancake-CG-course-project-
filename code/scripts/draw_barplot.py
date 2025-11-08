import os
import json
from collections import defaultdict
import argparse
import matplotlib.pyplot as plt
import re
import csv
import pandas as pd
import matplotlib

matplotlib.rcParams['hatch.linewidth'] = 0.05

fruits_translation = {
    "apple2": "Яблоко",
    "lemon": "Лимон",
    "banana": "Банан",
    "pear": "Груша",
}

def collect_raw_data(criterion_root: str):
    """
    Собирает сырые данные о времени выполнения для каждой группы и ее этапов.
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
    all_stage_names = sorted(list(set(stage for stages in all_data.values() for stage in stages.keys())))

    csv1_path = os.path.join(output_dir, 'timings_raw.csv')
    with open(csv1_path, 'w', newline='', encoding='utf-8') as f:
        writer = csv.writer(f)
        header = ['pair', 'total_time_ns'] + [f'{stage}_time_ns' for stage in all_stage_names]
        writer.writerow(header)

        for group_name, stages in all_data.items():
            total_time = sum(stages.values())
            row = [group_name, total_time] + [stages.get(s, 0) for s in all_stage_names]
            writer.writerow(row)

    print(f"Отчет с сырыми таймингами сохранен в '{csv1_path}'")

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
    return summary_for_chart, csv1_path


def create_percentage_stacked_bar_chart(csv_path: str, output_dir: str):
    """
    Создает 100% столбчатую диаграмму с накоплением и штриховкой.
    """
    try:
        df = pd.read_csv(csv_path)
        df['pair'] = df['pair'].apply(lambda p: " - ".join(fruits_translation[w] for w in p.split() if w in fruits_translation))
        df.set_index('pair', inplace=True)

        stage_columns = [col for col in df.columns if col.endswith('_time_ns') and col != 'total_time_ns']

        # Расчет процентов
        df_percent = df[stage_columns].div(df['total_time_ns'], axis=0) * 100
        df_percent = df_percent.reindex(df.sort_values(by='total_time_ns', ascending=False).index)

        # --- ПОДГОТОВКА К ПОСТРОЕНИЮ ГРАФИКА С ШТРИХОВКОЙ ---
        fig, ax = plt.subplots(figsize=(14, 8))

        # Определяем палитру цветов и стили штриховки
        # Вы можете добавить больше стилей, если у вас много этапов
        hatches = ["..", "++", "oo", "xx"]
        cmap = plt.get_cmap("tab20c")
        colors = [cmap(i) for i in range(len(stage_columns))]

        # Переменная для хранения высоты "дна" для следующего слоя
        bottom_values = pd.Series([0.0] * len(df_percent), index=df_percent.index)

        # --- ЦИКЛ ПОСТРОЕНИЯ КАЖДОГО СЛОЯ ---
        for i, stage_name in enumerate(stage_columns):
            # Получаем значения для текущего этапа
            values = df_percent[stage_name]
            # Рисуем слой столбиков
            ax.bar(
                df_percent.index,  # Метки по оси X (пары моделей)
                values,  # Высота сегментов этого слоя
                label=stage_name.replace('_time_ns', ''),  # Метка для легенды
                width=0.8,  # Ширина столбиков
                bottom=bottom_values,  # "Дно", на котором рисуется слой
                color=colors[i],  # Цвет заливки
                hatch=hatches[i % len(hatches)],  # Стиль штриховки (с зацикливанием)
                linewidth=0.5,
                edgecolor='white'
            )

            # Обновляем "дно" для следующего слоя
            bottom_values += values

        # --- НАСТРОЙКА ВНЕШНЕГО ВИДА (как и раньше) ---
        ax.set_title('Относительный вклад этапов в общее время морфинга', fontsize=16, weight='bold')
        ax.set_xlabel('Пара моделей', fontsize=12)
        ax.set_ylabel('Вклад в общее время (%)', fontsize=12)
        ax.set_ylim(0, 115)
        ax.yaxis.set_major_formatter(plt.FuncFormatter('{:.0f}%'.format))

        for i, pair_name in enumerate(df_percent.index):
            total_time_ns = df.loc[pair_name, 'total_time_ns']
            total_time_ms = total_time_ns / 1_000_000
            label_text = f"{total_time_ms:.1f} мс"
            ax.text(i, 102, label_text, ha='center', va='bottom', fontsize=9, weight='bold')

        ax.legend(title='Этапы', bbox_to_anchor=(1.02, 1), loc='upper left')

        # Устанавливаем тики и поворачиваем подписи для лучшей читаемости
        ax.set_xticks(ax.get_xticks())
        ax.set_xticklabels(df_percent.index, rotation=45, ha='right', rotation_mode='anchor')

        plt.grid(axis='y', linestyle='--', alpha=0.7)
        plt.tight_layout()

        chart_filename = os.path.join(output_dir, 'percentage_stacked_bar_chart.pdf')
        plt.savefig(chart_filename, format="pdf", bbox_inches='tight')
        print(f"Процентная столбчатая диаграмма сохранена в '{chart_filename}'")

    except Exception as e:
        print(f"Ошибка при создании процентной столбчатой диаграммы: {e}")


def main():
    parser = argparse.ArgumentParser(description="Анализ результатов Criterion.rs и генерация отчетов.")
    parser.add_argument("--root", default="target/criterion", help="Корневая директория с результатами Criterion.")
    parser.add_argument("--output", default="analysis_results", help="Директория для сохранения отчетов.")
    args = parser.parse_args()

    raw_data = collect_raw_data(args.root)
    if not raw_data: return

    summary_data, timings_csv_path = generate_reports(raw_data, args.output)

    if timings_csv_path:
        create_percentage_stacked_bar_chart(timings_csv_path, args.output)


if __name__ == "__main__":
    main()
