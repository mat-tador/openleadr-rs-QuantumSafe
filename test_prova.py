import subprocess
import time
import os
import matplotlib.pyplot as plt
import pandas as pd
import sys

# --- CONFIGURATION ---
OPENSSL_BIN = "/usr/local/openssl-pq/bin/openssl"
LIB_PATH = "/usr/local/openssl-pq/lib/ossl-modules"
ENV = os.environ.copy()
ENV["LD_LIBRARY_PATH"] = "/usr/local/lib:/usr/local/openssl-pq/lib"

# --- ALGORITHM LIST ---
algorithms = [
    # Legacy
    ("RSA-2048", "RSA", "Legacy (RSA)", ["-pkeyopt", "rsa_keygen_bits:2048"]),
    ("RSA-4096", "RSA", "Legacy (RSA)", ["-pkeyopt", "rsa_keygen_bits:4096"]),
    
    # Modern ECC
    ("ECC-P256", "EC", "Modern ECC", ["-pkeyopt", "ec_paramgen_curve:P-256"]),
    ("Ed25519", "ed25519", "Modern ECC", []), 
    ("X25519", "X25519", "Modern ECC", []),   

    # PQC Signatures
    ("Dilithium2 (L2)", "dilithium2", "PQC Signature", []),
    ("Dilithium3 (L3)", "dilithium3", "PQC Signature", []),
    ("Dilithium5 (L5)", "dilithium5", "PQC Signature", []),

    # PQC KEM
    ("ML-KEM-512", "ml-kem-512", "PQC KEM", []),
    ("ML-KEM-768", "ml-kem-768", "PQC KEM", []),
    ("ML-KEM-1024", "ml-kem-1024", "PQC KEM", []),
]

ITERATIONS = 100
results = []

print(f"{'Algorithm':<20} | {'Size (Bytes)':<12} | {'Avg Time (ms)':<15}")
print("-" * 55)

for name, algo_arg, category, extra_args in algorithms:
    key_file = f"key_{algo_arg}.pem"
    times = []
    
    cmd = [OPENSSL_BIN, "genpkey", "-algorithm", algo_arg]
    cmd.extend(extra_args)
    cmd.extend(["-out", key_file])
    
    if "PQC" in category:
        cmd.extend(["-provider-path", LIB_PATH, "-provider", "oqsprovider", "-provider", "default"])

    valid_size = 0
    
    sys.stdout.write(f"Benchmarking {name} ({ITERATIONS} loops)... \r")
    sys.stdout.flush()

    try:
        for _ in range(ITERATIONS):
            start_t = time.perf_counter()
            subprocess.run(cmd, env=ENV, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            end_t = time.perf_counter()
            times.append((end_t - start_t) * 1000)
            
        if os.path.exists(key_file):
            valid_size = os.path.getsize(key_file)
            os.remove(key_file)

    except Exception as e:
        print(f"\nSkipping {name}: {e}")
        continue

    avg_time = sum(times) / len(times)
    
    results.append({
        "Algorithm": name,
        "Category": category,
        "Size": valid_size,
        "Time": avg_time
    })
    
    sys.stdout.write("\033[K") 
    print(f"{name:<20} | {valid_size:<12} | {avg_time:<15.4f}")

# --- PLOTTING ---
if results:
    df = pd.DataFrame(results)
    
    plt.style.use('ggplot') 
    fig, ax = plt.subplots(figsize=(12, 8))
    
    category_colors = {
        'Legacy (RSA)': '#d62728',      # Red
        'Modern ECC': '#2ca02c',        # Green
        'PQC Signature': '#1f77b4',     # Blue
        'PQC KEM': '#9467bd'            # Purple
    }
    
    # 1. Scatter Plot
    for cat in df['Category'].unique():
        subset = df[df['Category'] == cat]
        ax.scatter(
            subset['Size'], 
            subset['Time'], 
            color=category_colors.get(cat, 'black'),
            s=220, # Slightly bigger dots
            label=cat,
            edgecolors='white',
            linewidth=1.5,
            alpha=0.95,
            zorder=2
        )

    # 2. Surgical Label Positioning
    for _, row in df.iterrows():
        x = row['Size']
        y = row['Time']
        label = row['Algorithm']
        
        # Default settings (Right center)
        xytext = (10, 0)
        ha = 'left'
        va = 'center'
        
        # --- LOGICA POSIZIONAMENTO SPECIFICA ---
        
        # RSA: Sempre a sinistra
        if "RSA" in label:
            xytext = (-12, 0); ha = 'right'
            
        # PQC KEM (Viola - Sopra a ventaglio)
        elif "ML-KEM-512" in label:
            xytext = (-15, 15); ha = 'right'; va = 'bottom'
        elif "ML-KEM-768" in label:
            xytext = (0, 20); ha = 'center'; va = 'bottom' # Più in alto
        elif "ML-KEM-1024" in label:
            xytext = (15, 15); ha = 'left'; va = 'bottom'

        # PQC Signature (Blu - Sotto a ventaglio) -> FIX PER DILITHIUM
        elif "Dilithium2" in label:
             # Sposta a sinistra e sotto
            xytext = (-15, -15); ha = 'right'; va = 'top'
        elif "Dilithium3" in label:
             # Sposta dritto sotto, molto più giù per liberare il punto
            xytext = (0, -25); ha = 'center'; va = 'top'
        elif "Dilithium5" in label:
             # Sposta a destra e sotto
            xytext = (15, -15); ha = 'left'; va = 'top'
            
        # ECC (Verde)
        elif "Ed25519" in label:
            xytext = (8, -15); ha = 'left'; va = 'top'
        elif "X25519" in label:
            xytext = (8, 15); ha = 'left'; va = 'bottom'
        # ECC-P256 usa il default

        # Disegna l'etichetta con sfondo per leggibilità
        ax.annotate(
            label,
            (x, y),
            xytext=xytext,
            textcoords='offset points',
            ha=ha, 
            va=va,
            fontsize=9,
            fontweight='medium',
            color='#222222',
            # Sfondo bianco semitrasparente migliorato
            bbox=dict(boxstyle="round,pad=0.3", fc="white", alpha=0.7, ec="#cccccc", lw=0.5)
        )

    # 3. Log Scales & Grid
    ax.set_yscale('log')
    ax.set_xscale('log')
    ax.grid(True, which='both', linestyle='--', linewidth=0.5)

    # 4. Labels & Legend
    ax.set_xlabel("Key File Size (Bytes) [Log Scale]", fontsize=12, fontweight='bold', color='#333333')
    ax.set_ylabel("Generation Time (ms) [Log Scale]", fontsize=12, fontweight='bold', color='#333333')
    
    legend = ax.legend(title="Algorithm Family", fontsize=10, loc='upper left', frameon=True, facecolor='white', framealpha=1, edgecolor='#cccccc')
    plt.setp(legend.get_title(), fontweight='bold')
    
    plt.tight_layout()
    
    output_filename = "benchmark_final_v4_perfect.png"
    plt.savefig(output_filename, dpi=300)
    print(f"\n[DONE] High-quality chart saved to: {output_filename}")

else:
    print("\n[ERROR] No data collected.")