#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""
Script to read Parquet files and create charts using Matplotlib
Creation date: April 22, 2025
"""

import os
import sys
import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.dates as mdates
from matplotlib.ticker import FuncFormatter
import seaborn as sns
import argparse
from datetime import datetime
import dask.dataframe as dd

def load_data(file_path):
    """
    Read a Parquet file and return it as a DataFrame.
    Uses Dask for efficient loading of large Parquet files.
    
    Args:
        file_path (str): Path to the Parquet file
        
    Returns:
        pandas.DataFrame: DataFrame (converted from Dask DataFrame)
    """
    if not os.path.exists(file_path):
        print(f"Error: File does not exist - {file_path}")
        sys.exit(1)
    
    try:
        # Use Dask to read the Parquet file
        print(f"Loading file {file_path} using Dask...")
        ddf = dd.read_parquet(file_path)
        
        # Get basic info before computation
        print(f"File loaded as Dask DataFrame with {len(ddf.divisions)-1} partitions")
        
        # Convert to pandas DataFrame for visualization
        # For very large files, you might want to use ddf.sample() or apply operations in Dask first
        df = ddf.compute()
        print(f"Successfully converted to pandas DataFrame")
        print(f"Data shape: {df.shape}")
        return df
    except Exception as e:
        print(f"Error occurred while reading file: {e}")
        sys.exit(1)

def create_subplot_figure():
    """
    Create a large figure with subplots to display all charts.
    
    Returns:
        tuple: (fig, axs) - matplotlib figure object and axes array
    """
    # Create a large chart with a 4x3 grid
    fig, axs = plt.subplots(4, 3, figsize=(24, 16))
    # Convert to a 1D array for ease of use
    axs = axs.flatten()
    # Adjust margins
    fig.tight_layout(pad=4.0)
    return fig, axs

def plot_time_series(df, x_col, y_col, title, ylabel, ax, color='blue', marker='.', alpha=0.7):
    """
    Create a time-based scatter plot.
    
    Args:
        df (pandas.DataFrame): DataFrame
        x_col (str): X-axis column (usually time)
        y_col (str): Y-axis column
        title (str): Chart title
        ylabel (str): Y-axis label
        ax (matplotlib.axes): Axes object to draw the graph
        color (str): Point color
        marker (str): Marker style
        alpha (float): Transparency
    """
    ax.scatter(df[x_col], df[y_col], c=color, marker=marker, alpha=alpha)
    ax.set_title(title, fontsize=10)
    ax.set_xlabel('Time (seconds)', fontsize=8)
    ax.set_ylabel(ylabel, fontsize=8)
    ax.grid(True, alpha=0.3)
    ax.tick_params(axis='both', which='major', labelsize=8)

def plot_histogram(df, col, title, xlabel, ax, bins=50, color='blue', alpha=0.7):
    """
    Create a histogram.
    
    Args:
        df (pandas.DataFrame): DataFrame
        col (str): Column to represent as a histogram
        title (str): Chart title
        xlabel (str): X-axis label
        ax (matplotlib.axes): Axes object to draw the graph
        bins (int): Number of histogram bins
        color (str): Bar color
        alpha (float): Transparency
    """
    ax.hist(df[col], bins=bins, color=color, alpha=alpha)
    ax.set_title(title, fontsize=10)
    ax.set_xlabel(xlabel, fontsize=8)
    ax.set_ylabel('Frequency', fontsize=8)
    ax.grid(True, alpha=0.3)
    ax.tick_params(axis='both', which='major', labelsize=8)

def plot_pie(df, col, title, ax):
    """
    Create a pie chart.
    
    Args:
        df (pandas.DataFrame): DataFrame
        col (str): Column to represent as a pie chart
        title (str): Chart title
        ax (matplotlib.axes): Axes object to draw the graph
    """
    # Aggregate data
    counts = df[col].value_counts()
    labels = counts.index.tolist()
    if isinstance(labels[0], bool):
        labels = ['Continuous' if x else 'Non-continuous' for x in labels]
    
    wedges, texts, autotexts = ax.pie(counts.values, labels=labels, autopct='%1.1f%%', 
                                    textprops={'fontsize': 8}, startangle=90)
    for text in texts:
        text.set_fontsize(8)
    for autotext in autotexts:
        autotext.set_fontsize(8)
    
    ax.set_title(title, fontsize=10)

def plot_opcode_distribution(df, ax):
    """
    Represent the opcode distribution of UFS data as a bar graph.
    
    Args:
        df (pandas.DataFrame): UFS DataFrame
        ax (matplotlib.axes): Axes object to draw the graph
    """
    if 'opcode' not in df.columns:
        return
    
    opcode_counts = df['opcode'].value_counts()
    
    sns.barplot(x=opcode_counts.index, y=opcode_counts.values, ax=ax)
    ax.set_title('Opcode Distribution', fontsize=10)
    ax.set_xlabel('Opcode', fontsize=8)
    ax.set_ylabel('Frequency', fontsize=8)
    ax.tick_params(axis='x', rotation=45, labelsize=8)
    ax.tick_params(axis='y', labelsize=8)

def plot_io_type_distribution(df, ax):
    """
    Represent the IO type distribution of Block data as a bar graph.
    
    Args:
        df (pandas.DataFrame): Block DataFrame
        ax (matplotlib.axes): Axes object to draw the graph
    """
    if 'io_type' not in df.columns:
        return
    
    io_type_counts = df['io_type'].value_counts()
    
    sns.barplot(x=io_type_counts.index, y=io_type_counts.values, ax=ax)
    ax.set_title('I/O Type Distribution', fontsize=10)
    ax.set_xlabel('I/O Type', fontsize=8)
    ax.set_ylabel('Frequency', fontsize=8)
    ax.tick_params(axis='both', which='major', labelsize=8)

def plot_by_operation_type(df, x_col, y_col, category_col, title, xlabel, ylabel, output_path, is_ufs=True):
    """
    Create a chart with different operation types (opcode for UFS, io_type for Block)
    as separate series with different colors.
    
    Args:
        df (pandas.DataFrame): DataFrame
        x_col (str): X-axis column (usually time)
        y_col (str): Y-axis column (latency values)
        category_col (str): Column to use for categorization (opcode or io_type)
        title (str): Chart title
        xlabel (str): X-axis label
        ylabel (str): Y-axis label
        output_path (str): Output file path
        is_ufs (bool): If True, process as UFS data, otherwise as Block data
    """
    plt.figure(figsize=(12, 6))
    
    # Get unique categories
    categories = df[category_col].unique()
    
    # Create a color palette
    palette = sns.color_palette("husl", len(categories))
    
    # Plot each category as a separate series
    for i, category in enumerate(categories):
        category_df = df[df[category_col] == category]
        plt.scatter(category_df[x_col], category_df[y_col], 
                   label=category, alpha=0.7, edgecolor='w', linewidth=0.5,
                   c=[palette[i]])
    
    plt.title(title)
    plt.xlabel(xlabel)
    plt.ylabel(ylabel)
    plt.grid(True, alpha=0.3)
    plt.legend(title=category_col, loc='best', frameon=True, framealpha=0.8)
    
    # Add a horizontal line at y=0 for reference
    plt.axhline(y=0, color='grey', linestyle='--', alpha=0.7)
    
    # If there are too many points, limit the view to show outliers better
    if len(df) > 1000:
        y_values = df[y_col].values
        q1, q3 = np.percentile(y_values, [25, 75])
        iqr = q3 - q1
        upper_limit = q3 + 5 * iqr
        lower_limit = q1 - 5 * iqr
        plt.ylim(max(lower_limit, min(y_values)), min(upper_limit, max(y_values)))
    
    plt.tight_layout()
    plt.savefig(output_path, dpi=300)
    plt.close()
    print(f"Chart saved: {output_path}")

def visualize_issue_send_based(df, output_dir, is_ufs=True):
    """
    Create charts focused on issue/send operations with opcode/io_type as legend.
    
    Args:
        df (pandas.DataFrame): DataFrame (UFS or Block)
        output_dir (str): Output directory
        is_ufs (bool): If True, process as UFS data, otherwise as Block data
    """
    os.makedirs(output_dir, exist_ok=True)
    
    category_col = 'opcode' if is_ufs else 'io_type'
    data_type = 'UFS' if is_ufs else 'Block I/O'
    
    # Make sure the category column exists
    if category_col not in df.columns:
        print(f"Warning: {category_col} column not found in {data_type} data")
        return
    
    # 1. DTOC by operation type (Dispatch to Complete)
    plot_by_operation_type(
        df, 'time', 'dtoc', category_col,
        f'{data_type} Dispatch to Complete Latency by {category_col}',
        'Time (seconds)', 'Dispatch to Complete Latency (ms)',
        os.path.join(output_dir, f'{"ufs" if is_ufs else "block"}_dtoc_by_{category_col}.png'),
        is_ufs
    )
    
    # 2. CTOD by operation type (Complete to Dispatch) - this is complete-based
    plot_by_operation_type(
        df, 'time', 'ctod', category_col,
        f'{data_type} Complete to Dispatch Latency by {category_col}',
        'Time (seconds)', 'Complete to Dispatch Latency (ms)',
        os.path.join(output_dir, f'{"ufs" if is_ufs else "block"}_ctod_by_{category_col}.png'),
        is_ufs
    )
    
    # 3. CTOC by operation type (Complete to Complete)
    plot_by_operation_type(
        df, 'time', 'ctoc', category_col,
        f'{data_type} Complete to Complete Latency by {category_col}',
        'Time (seconds)', 'Complete to Complete Latency (ms)',
        os.path.join(output_dir, f'{"ufs" if is_ufs else "block"}_ctoc_by_{category_col}.png'),
        is_ufs
    )
    
    # 4. Queue Depth by operation type
    plot_by_operation_type(
        df, 'time', 'qd', category_col,
        f'{data_type} Queue Depth by {category_col}',
        'Time (seconds)', 'Queue Depth',
        os.path.join(output_dir, f'{"ufs" if is_ufs else "block"}_qd_by_{category_col}.png'),
        is_ufs
    )
    
    print(f"{data_type} issue/send-based visualization with {category_col} legend completed.")

def visualize_ufs_data_combined(df, output_path):
    """
    Visualize UFS data in a single PNG file.
    
    Args:
        df (pandas.DataFrame): UFS DataFrame
        output_path (str): Output file path
    """
    # Create full chart and subplots
    fig, axs = create_subplot_figure()
    
    # Set chart title
    fig.suptitle('UFS Data Comprehensive Analysis', fontsize=20)
    
    # Draw charts on each subplot
    # 1. LBA distribution over time
    plot_time_series(df, 'time', 'lba', 'LBA Distribution over Time', 'LBA', axs[0])
    
    # 2. Queue Depth distribution over time
    plot_time_series(df, 'time', 'qd', 'Queue Depth Distribution over Time', 'Queue Depth', axs[1])
    
    # 3. Dispatch to Complete latency over time
    plot_time_series(df, 'time', 'dtoc', 'Dispatch to Complete Latency over Time', 
                    'Latency (ms)', axs[2], color='green')
    
    # 4. Complete to Dispatch latency over time
    plot_time_series(df, 'time', 'ctod', 'Complete to Dispatch Latency over Time', 
                    'Latency (ms)', axs[3], color='orange')
    
    # 5. Complete to Complete latency over time
    plot_time_series(df, 'time', 'ctoc', 'Complete to Complete Latency over Time', 
                    'Latency (ms)', axs[4], color='red')
    
    # 6. Latency (dtoc) histogram
    plot_histogram(df, 'dtoc', 'Dispatch to Complete Latency Distribution', 
                 'Latency (ms)', axs[5], color='green')
    
    # 7. Latency (ctod) histogram
    plot_histogram(df, 'ctod', 'Complete to Dispatch Latency Distribution', 
                 'Latency (ms)', axs[6], color='orange')
    
    # 8. Latency (ctoc) histogram
    plot_histogram(df, 'ctoc', 'Complete to Complete Latency Distribution', 
                 'Latency (ms)', axs[7], color='red')
    
    # 9. Continuity pie chart
    plot_pie(df, 'continuous', 'UFS Continuity Distribution', axs[8])
    
    # 10. Opcode distribution
    plot_opcode_distribution(df, axs[9])
    
    # 11. Latency summary statistics
    ax_stats = axs[10]
    dtoc_stats = df['dtoc'].describe()
    ctod_stats = df['ctod'].describe()
    ctoc_stats = df['ctoc'].describe()
    
    stats_table = pd.DataFrame({
        'DTOC (ms)': dtoc_stats,
        'Complete to Dispatch (ms)': ctod_stats,
        'CTOC (ms)': ctoc_stats
    })
    
    ax_stats.axis('tight')
    ax_stats.axis('off')
    table = ax_stats.table(cellText=stats_table.round(3).values,
                          rowLabels=stats_table.index,
                          colLabels=stats_table.columns,
                          cellLoc='center',
                          loc='center')
    table.auto_set_font_size(False)
    table.set_fontsize(8)
    table.scale(1, 1.5)
    ax_stats.set_title('Latency Summary Statistics', fontsize=10)
    
    # 12. Process statistics
    ax_process = axs[11]
    top_processes = df.groupby('process')['dtoc'].mean().nlargest(10)
    sns.barplot(x=top_processes.index, y=top_processes.values, ax=ax_process)
    ax_process.set_title('Average DTOC Latency by Process (Top 10)', fontsize=10)
    ax_process.set_xlabel('Process', fontsize=8)
    ax_process.set_ylabel('Average Latency (ms)', fontsize=8)
    ax_process.tick_params(axis='x', rotation=45, labelsize=7)
    
    # Final layout adjustment
    plt.tight_layout(rect=[0, 0, 1, 0.97])  # Reserve space for title
    
    # Save file
    plt.savefig(output_path, dpi=300)
    plt.close(fig)
    print(f"Chart saved: {output_path}")

def visualize_block_data_combined(df, output_path):
    """
    Visualize Block data in a single PNG file.
    
    Args:
        df (pandas.DataFrame): Block DataFrame
        output_path (str): Output file path
    """
    # Create full chart and subplots
    fig, axs = create_subplot_figure()
    
    # Set chart title
    fig.suptitle('Block I/O Data Comprehensive Analysis', fontsize=20)
    
    # Draw charts on each subplot
    # 1. Sector distribution over time
    plot_time_series(df, 'time', 'sector', 'Sector Distribution over Time', 'Sector', axs[0])
    
    # 2. Queue Depth distribution over time
    plot_time_series(df, 'time', 'qd', 'Queue Depth Distribution over Time', 'Queue Depth', axs[1])
    
    # 3. Dispatch to Complete latency over time
    plot_time_series(df, 'time', 'dtoc', 'Dispatch to Complete Latency over Time', 
                    'Latency (ms)', axs[2], color='green')
    
    # 4. Complete to Dispatch latency over time
    plot_time_series(df, 'time', 'ctod', 'Complete to Dispatch Latency over Time', 
                    'Latency (ms)', axs[3], color='orange')
    
    # 5. Complete to Complete latency over time
    plot_time_series(df, 'time', 'ctoc', 'Complete to Complete Latency over Time', 
                    'Latency (ms)', axs[4], color='red')
    
    # 6. Latency (dtoc) histogram
    plot_histogram(df, 'dtoc', 'Dispatch to Complete Latency Distribution', 
                 'Latency (ms)', axs[5], color='green')
    
    # 7. Latency (ctod) histogram
    plot_histogram(df, 'ctod', 'Complete to Dispatch Latency Distribution', 
                 'Latency (ms)', axs[6], color='orange')
    
    # 8. Latency (ctoc) histogram
    plot_histogram(df, 'ctoc', 'Complete to Complete Latency Distribution', 
                 'Latency (ms)', axs[7], color='red')
    
    # 9. Continuity pie chart
    plot_pie(df, 'continuous', 'Block I/O Continuity Distribution', axs[8])
    
    # 10. I/O type distribution
    plot_io_type_distribution(df, axs[9])
    
    # 11. Latency summary statistics
    ax_stats = axs[10]
    dtoc_stats = df['dtoc'].describe()
    ctod_stats = df['ctod'].describe()
    ctoc_stats = df['ctoc'].describe()
    
    stats_table = pd.DataFrame({
        'DTOC (ms)': dtoc_stats,
        'Complete to Dispatch (ms)': ctod_stats,
        'CTOC (ms)': ctoc_stats
    })
    
    ax_stats.axis('tight')
    ax_stats.axis('off')
    table = ax_stats.table(cellText=stats_table.round(3).values,
                          rowLabels=stats_table.index,
                          colLabels=stats_table.columns,
                          cellLoc='center',
                          loc='center')
    table.auto_set_font_size(False)
    table.set_fontsize(8)
    table.scale(1, 1.5)
    ax_stats.set_title('Latency Summary Statistics', fontsize=10)
    
    # 12. Process statistics
    ax_process = axs[11]
    top_processes = df.groupby('process')['dtoc'].mean().nlargest(10)
    sns.barplot(x=top_processes.index, y=top_processes.values, ax=ax_process)
    ax_process.set_title('Average DTOC Latency by Process (Top 10)', fontsize=10)
    ax_process.set_xlabel('Process', fontsize=8)
    ax_process.set_ylabel('Average Latency (ms)', fontsize=8)
    ax_process.tick_params(axis='x', rotation=45, labelsize=7)
    
    # Final layout adjustment
    plt.tight_layout(rect=[0, 0, 1, 0.97])  # Reserve space for title
    
    # Save file
    plt.savefig(output_path, dpi=300)
    plt.close(fig)
    print(f"Chart saved: {output_path}")

def visualize_ufs_data(df, output_dir):
    """
    Perform visualization for UFS data.
    
    Args:
        df (pandas.DataFrame): UFS DataFrame
        output_dir (str): Output directory
    """
    os.makedirs(output_dir, exist_ok=True)
    
    # 1. LBA distribution over time
    plot_time_series(
        df, 'time', 'lba', 
        'LBA Distribution over Time', 'LBA', 
        os.path.join(output_dir, 'ufs_lba_time.png')
    )
    
    # 2. Queue Depth distribution over time
    plot_time_series(
        df, 'time', 'qd', 
        'Queue Depth Distribution over Time', 'Queue Depth', 
        os.path.join(output_dir, 'ufs_qd_time.png')
    )
    
    # 3. Dispatch to Complete latency over time
    plot_time_series(
        df, 'time', 'dtoc', 
        'Dispatch to Complete Latency over Time', 'Latency (ms)', 
        os.path.join(output_dir, 'ufs_dtoc_time.png'), 
        color='green'
    )
    
    # 4. Complete to Dispatch latency over time
    plot_time_series(
        df, 'time', 'ctod', 
        'Complete to Dispatch Latency over Time', 'Latency (ms)', 
        os.path.join(output_dir, 'ufs_ctod_time.png'), 
        color='orange'
    )
    
    # 5. Complete to Complete latency over time
    plot_time_series(
        df, 'time', 'ctoc', 
        'Complete to Complete Latency over Time', 'Latency (ms)', 
        os.path.join(output_dir, 'ufs_ctoc_time.png'), 
        color='red'
    )
    
    # 6. Latency (dtoc) histogram
    plot_histogram(
        df, 'dtoc', 
        'Dispatch to Complete Latency Distribution', 'Latency (ms)', 
        os.path.join(output_dir, 'ufs_dtoc_hist.png'),
        color='green'
    )
    
    # 7. Latency (ctod) histogram
    plot_histogram(
        df, 'ctod', 
        'Complete to Dispatch Latency Distribution', 'Latency (ms)', 
        os.path.join(output_dir, 'ufs_ctod_hist.png'),
        color='orange'
    )
    
    # 8. Latency (ctoc) histogram
    plot_histogram(
        df, 'ctoc', 
        'Complete to Complete Latency Distribution', 'Latency (ms)', 
        os.path.join(output_dir, 'ufs_ctoc_hist.png'),
        color='red'
    )
    
    # 9. Continuity pie chart
    plot_pie(
        df, 'continuous', 
        'UFS Continuity Distribution', 
        os.path.join(output_dir, 'ufs_continuous_pie.png')
    )
    
    # 10. Opcode distribution
    plot_opcode_distribution(
        df, 
        os.path.join(output_dir, 'ufs_opcode_dist.png')
    )
    
    # 11. Process-time DTOC heatmap
    plot_heatmap(
        df, 
        os.path.join(output_dir, 'ufs_process_time_dtoc_heatmap.png'),
        'dtoc'
    )
    
    print("UFS data visualization completed.")

def visualize_block_data(df, output_dir):
    """
    Perform visualization for Block data.
    
    Args:
        df (pandas.DataFrame): Block DataFrame
        output_dir (str): Output directory
    """
    os.makedirs(output_dir, exist_ok=True)
    
    # 1. Sector distribution over time
    plot_time_series(
        df, 'time', 'sector', 
        'Sector Distribution over Time', 'Sector', 
        os.path.join(output_dir, 'block_sector_time.png')
    )
    
    # 2. Queue Depth distribution over time
    plot_time_series(
        df, 'time', 'qd', 
        'Queue Depth Distribution over Time', 'Queue Depth', 
        os.path.join(output_dir, 'block_qd_time.png')
    )
    
    # 3. Dispatch to Complete latency over time
    plot_time_series(
        df, 'time', 'dtoc', 
        'Dispatch to Complete Latency over Time', 'Latency (ms)', 
        os.path.join(output_dir, 'block_dtoc_time.png'), 
        color='green'
    )
    
    # 4. Complete to Dispatch latency over time
    plot_time_series(
        df, 'time', 'ctod', 
        'Complete to Dispatch Latency over Time', 'Latency (ms)', 
        os.path.join(output_dir, 'block_ctod_time.png'), 
        color='orange'
    )
    
    # 5. Complete to Complete latency over time
    plot_time_series(
        df, 'time', 'ctoc', 
        'Complete to Complete Latency over Time', 'Latency (ms)', 
        os.path.join(output_dir, 'block_ctoc_time.png'), 
        color='red'
    )
    
    # 6. Latency (dtoc) histogram
    plot_histogram(
        df, 'dtoc', 
        'Dispatch to Complete Latency Distribution', 'Latency (ms)', 
        os.path.join(output_dir, 'block_dtoc_hist.png'),
        color='green'
    )
    
    # 7. Latency (ctod) histogram
    plot_histogram(
        df, 'ctod', 
        'Complete to Dispatch Latency Distribution', 'Latency (ms)', 
        os.path.join(output_dir, 'block_ctod_hist.png'),
        color='orange'
    )
    
    # 8. Latency (ctoc) histogram
    plot_histogram(
        df, 'ctoc', 
        'Complete to Complete Latency Distribution', 'Latency (ms)', 
        os.path.join(output_dir, 'block_ctoc_hist.png'),
        color='red'
    )
    
    # 9. Continuity pie chart
    plot_pie(
        df, 'continuous', 
        'Block I/O Continuity Distribution', 
        os.path.join(output_dir, 'block_continuous_pie.png')
    )
    
    # 10. I/O type distribution
    plot_io_type_distribution(
        df, 
        os.path.join(output_dir, 'block_io_type_dist.png')
    )
    
    # 11. Process-time DTOC heatmap
    plot_heatmap(
        df, 
        os.path.join(output_dir, 'block_process_time_dtoc_heatmap.png'),
        'dtoc'
    )
    
    print("Block data visualization completed.")

def plot_heatmap(df, output_path, target_col='dtoc'):
    """
    Create a heatmap of key metrics by time period and process.
    
    Args:
        df (pandas.DataFrame): DataFrame
        output_path (str): Output file path
        target_col (str): Column for values to be represented in the heatmap
    """
    # Divide time into 10 bins and calculate average by process
    df['time_bin'] = pd.cut(df['time'], 10)
    pivot = df.pivot_table(
        index='process', 
        columns='time_bin', 
        values=target_col, 
        aggfunc='mean'
    ).fillna(0)
    
    # If there are too many processes, select only the top 10
    if len(pivot) > 10:
        row_sums = pivot.sum(axis=1)
        pivot = pivot.loc[row_sums.nlargest(10).index]
    
    plt.figure(figsize=(14, 10))
    sns.heatmap(pivot, cmap='viridis', annot=False, fmt=".2f")
    plt.title(f'Average {target_col} by Process (Time Period)')
    plt.xlabel('Time Period')
    plt.ylabel('Process')
    
    # Save file
    plt.tight_layout()
    plt.savefig(output_path)
    plt.close()
    print(f"Chart saved: {output_path}")

def main():
    """
    Main function
    """
    parser = argparse.ArgumentParser(description='Read Parquet files and create charts with Matplotlib')
    parser.add_argument('parquet_file', help='Parquet file path (enter prefix only, _ufs.parquet and _block.parquet will be added automatically)')
    parser.add_argument('--output-dir', '-o', default='./plots', help='Output directory (default: ./plots)')
    parser.add_argument('--combined', '-c', action='store_true', help='Create all charts in a single PNG file (default: False)')
    parser.add_argument('--sample', '-s', type=int, default=0, help='Sample size to use (default: 0, use all data)')
    parser.add_argument('--issue-send', '-i', action='store_true', help='Create issue/send based charts with opcode/io_type as legend')
    
    args = parser.parse_args()
    
    # Create output directory based on current time
    timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
    output_dir = os.path.join(args.output_dir, timestamp)
    os.makedirs(output_dir, exist_ok=True)
    
    # Load UFS data file and visualize
    ufs_file = f"{args.parquet_file}_ufs.parquet"
    if os.path.exists(ufs_file):
        ufs_df = load_data(ufs_file)
        
        # Sample data if requested
        if args.sample > 0 and len(ufs_df) > args.sample:
            print(f"Sampling {args.sample} rows from UFS data...")
            ufs_df = ufs_df.sample(n=args.sample, random_state=42)
        
        if args.issue_send:
            # Create issue/send based charts with opcode as legend
            visualize_issue_send_based(ufs_df, os.path.join(output_dir, 'ufs_issue_send'), is_ufs=True)
        elif args.combined:
            visualize_ufs_data_combined(ufs_df, os.path.join(output_dir, 'ufs_combined.png'))
        else:
            visualize_ufs_data(ufs_df, os.path.join(output_dir, 'ufs'))
    else:
        print(f"Warning: UFS file not found - {ufs_file}")
    
    # Load Block data file and visualize
    block_file = f"{args.parquet_file}_block.parquet"
    if os.path.exists(block_file):
        block_df = load_data(block_file)
        
        # Sample data if requested
        if args.sample > 0 and len(block_df) > args.sample:
            print(f"Sampling {args.sample} rows from Block data...")
            block_df = block_df.sample(n=args.sample, random_state=42)
        
        if args.issue_send:
            # Create issue/send based charts with io_type as legend
            visualize_issue_send_based(block_df, os.path.join(output_dir, 'block_issue_send'), is_ufs=False)
        elif args.combined:
            visualize_block_data_combined(block_df, os.path.join(output_dir, 'block_combined.png'))
        else:
            visualize_block_data(block_df, os.path.join(output_dir, 'block'))
    else:
        print(f"Warning: Block file not found - {block_file}")
    
    print(f"All charts have been saved to the {output_dir} directory.")

if __name__ == "__main__":
    main()