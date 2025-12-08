import pandas as pd
import seaborn as sns
import matplotlib.pyplot as plt
import os

# --- 1. CONFIGURAZIONE ESTETICA ---
sns.set_theme(style="ticks", rc={"axes.grid": True, "grid.linestyle": ":", "axes.spines.right": False, "axes.spines.top": False})
plt.rcParams['font.family'] = 'sans-serif'
plt.rcParams['font.sans-serif'] = ['Arial', 'DejaVu Sans', 'Liberation Sans']
text_color = '#555555'
PALETTE = {"ECC (P-256)": "#4A90E2", "Dilithium3": "#E74C3C"}

def load_and_prep(file_ecc, file_dil):
    """Carica i CSV e prepara i dati."""
    data_frames = []
    
    # Carica ECC
    if os.path.exists(file_ecc):
        try:
            # Prende solo i primi 100 per coerenza
            df = pd.read_csv(file_ecc, header=None, names=['ID', 'Time_Micro'], nrows=100)
            df['Algorithm'] = 'ECC (P-256)'
            data_frames.append(df)
        except pd.errors.EmptyDataError:
             pass

    # Carica Dilithium
    if os.path.exists(file_dil):
        try:
            df = pd.read_csv(file_dil, header=None, names=['ID', 'Time_Micro'], nrows=100)
            df['Algorithm'] = 'Dilithium3'
            data_frames.append(df)
        except pd.errors.EmptyDataError:
             pass

    if not data_frames:
        return pd.DataFrame()

    combined = pd.concat(data_frames)
    combined['Time_ms'] = combined['Time_Micro'] / 1000.0 # Converti in ms
    return combined

def create_and_save_plot(data, filename, y_label_text):
    """Genera e salva il grafico boxplot."""
    if data.empty:
        return

    plt.figure(figsize=(8, 7))
    ax = plt.gca()

    sns.boxplot(
        data=data, x='Algorithm', y='Time_ms', palette=PALETTE, 
        width=0.4, linewidth=1.5, fliersize=4, 
        flierprops={"marker": "o", "markerfacecolor": "none", "markeredgecolor": text_color, "alpha": 0.6}
    )

    ax.set_xlabel("")
    ax.set_ylabel(y_label_text, fontsize=12, fontweight='bold', color=text_color)
    ax.tick_params(axis='x', labelsize=12, colors=text_color)
    ax.tick_params(axis='y', labelsize=10, colors=text_color)
    
    if data['Time_ms'].min() > 0: ax.set_ylim(bottom=0)

    # Annotazione "Coda" (Whisker)
    ax.annotate(
        'The "Tail" (Whisker):\nExtends to 1.5x IQR.\nPoints beyond are outliers.',
        xy=(0.75, 0.75), xycoords='axes fraction',
        xytext=(0.3, 0.9), textcoords='axes fraction',
        arrowprops=dict(facecolor=text_color, shrink=0.05, width=1, headwidth=6, alpha=0.7),
        fontsize=9, color=text_color, ha='left', va='center',
        bbox=dict(boxstyle="round,pad=0.3", fc="white", ec="#dddddd", lw=1)
    )

    sns.despine(trim=True)
    plt.tight_layout()
    plt.savefig(filename, dpi=300, bbox_inches='tight')
    plt.close()
    print(f"🖼️  Grafico salvato: {filename}")

def print_stats(data, operation_name):
    """Calcola e stampa le statistiche a terminale."""
    if data.empty:
        print(f"⚠️  Dati mancanti per {operation_name}")
        return

    # Calcola le medie per algoritmo
    means = data.groupby('Algorithm')['Time_ms'].mean()
    
    ecc_mean = means.get('ECC (P-256)')
    dil_mean = means.get('Dilithium3')

    print(f"\n📊 --- STATISTICHE {operation_name.upper()} ---")
    
    if ecc_mean is not None:
        print(f"   🔹 ECC (P-256) Media:   {ecc_mean:.4f} ms")
    else:
        print("   🔹 ECC (P-256): Dati mancanti")

    if dil_mean is not None:
        print(f"   🔹 Dilithium3 Media:    {dil_mean:.4f} ms")
    else:
        print("   🔹 Dilithium3:  Dati mancanti")

    # Calcola Ratio
    if ecc_mean and dil_mean and ecc_mean > 0:
        ratio = dil_mean / ecc_mean
        print(f"   🚀 SPEED RATIO: Dilithium è {ratio:.2f}x più LENTO di ECC")
    
    print("-" * 40)


# --- ESECUZIONE ---

print("Caricamento dati...\n")

# 1. FIRMA (Signing)
df_sign = load_and_prep('ECC_signing_time.csv', 'DILITHIUM_signing_time.csv')
create_and_save_plot(df_sign, "signing_benchmark.png", "Signing Time (ms)")
print_stats(df_sign, "Signing")

# 2. VERIFICA (Verification)
df_verify = load_and_prep('ECC_verification_time.csv', 'DILITHIUM_verification_time.csv')
create_and_save_plot(df_verify, "verification_benchmark.png", "Verification Time (ms)")
print_stats(df_verify, "Verification")

print("\n✅ Tutto fatto.")