| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `ouch compress rust output.tar` | 445.3 ± 5.8 | 436.3 | 453.4 | 1.43 ± 0.05 |
| `tar -cvf output.tar rust` | 311.6 ± 10.8 | 304.0 | 339.8 | 1.00 |

| Command | Mean [s] | Min [s] | Max [s] | Relative |
|:---|---:|---:|---:|---:|
| `ouch decompress input.tar --dir output` | 1.393 ± 0.031 | 1.371 | 1.474 | 1.85 ± 0.05 |
| `tar -xv -C output -f input.tar` | 0.751 ± 0.010 | 0.738 | 0.770 | 1.00 |

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `ouch compress compiler output.tar.gz` | 667.9 ± 8.7 | 657.1 | 680.6 | 1.00 ± 0.02 |
| `tar -cvzf output.tar.gz compiler` | 667.8 ± 8.8 | 656.9 | 685.3 | 1.00 |

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `ouch decompress input.tar.gz --dir output` | 170.7 ± 2.0 | 165.8 | 173.1 | 1.25 ± 0.03 |
| `tar -xvz -C output -f input.tar.gz` | 136.2 ± 2.2 | 132.8 | 141.1 | 1.00 |

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `ouch compress compiler output.zip` | 549.7 ± 4.3 | 543.6 | 558.6 | 1.00 |
| `zip output.zip -r compiler` | 581.3 ± 9.1 | 573.2 | 600.9 | 1.06 ± 0.02 |

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `ouch decompress input.zip --dir output` | 171.3 ± 2.4 | 166.9 | 174.3 | 1.00 |
| `unzip input.zip -d output` | 218.3 ± 4.9 | 211.9 | 229.3 | 1.27 ± 0.03 |
