/// Mesure de la VM WSL : mémoire, temps processeur cumulé, nombre de coeurs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VmSample {
    pub ram_bytes: u64,
    pub cpu_secs: f64,
    pub cores: u32,
}

/// Script PowerShell dont la sortie est lue par `parse_vm_sample` ("ram cpu coeurs").
pub const VM_SCRIPT: &str = "$p=@(Get-Process -Name vmmem,vmmemWSL -ErrorAction SilentlyContinue); \
$r=0;$c=0.0; foreach($x in $p){$r+=$x.WorkingSet64;$c+=$x.CPU}; \
[string]::Format([Globalization.CultureInfo]::InvariantCulture,'{0} {1} {2}',$r,$c,$env:NUMBER_OF_PROCESSORS)";

pub fn parse_vm_sample(text: &str) -> Option<VmSample> {
    let mut it = text.split_whitespace();
    let ram_bytes = it.next()?.parse().ok()?;
    let cpu_secs = it.next()?.parse().ok()?;
    let cores = it.next()?.parse().ok()?;
    Some(VmSample {
        ram_bytes,
        cpu_secs,
        cores,
    })
}

/// Pourcentage CPU de la VM entre deux mesures espacées de `elapsed_secs`.
pub fn cpu_percent(prev_cpu: f64, cur: &VmSample, elapsed_secs: f64) -> Option<f64> {
    if elapsed_secs <= 0.0 || cur.cores == 0 || cur.cpu_secs < prev_cpu {
        return None;
    }
    let pct = (cur.cpu_secs - prev_cpu) / (elapsed_secs * f64::from(cur.cores)) * 100.0;
    Some(pct.clamp(0.0, 100.0))
}

/// Analyse `df -k /` : renvoie (utilisé, total) en octets.
pub fn parse_df(text: &str) -> Option<(u64, u64)> {
    let line = text
        .lines()
        .rfind(|l| !l.trim().is_empty() && !l.starts_with("Filesystem"))?;
    let f: Vec<&str> = line.split_whitespace().collect();
    if f.len() < 4 {
        return None;
    }
    let total: u64 = f[1].parse().ok()?;
    let used: u64 = f[2].parse().ok()?;
    Some((used * 1024, total * 1024))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echantillon_vm() {
        let s = parse_vm_sample("3435973836 12.5 8\r\n").unwrap();
        assert_eq!((s.ram_bytes, s.cores), (3435973836, 8));
        assert!(parse_vm_sample("").is_none());
        assert!(parse_vm_sample("a b c").is_none());
    }

    #[test]
    fn cpu() {
        let s = VmSample {
            ram_bytes: 0,
            cpu_secs: 14.0,
            cores: 4,
        };
        assert_eq!(cpu_percent(10.0, &s, 5.0), Some(20.0));
        assert_eq!(cpu_percent(20.0, &s, 5.0), None);
        assert_eq!(cpu_percent(0.0, &s, 0.0), None);
    }

    #[test]
    fn df() {
        let t = "Filesystem     1K-blocks     Used Available Use% Mounted on\n/dev/sdc      1055762868 12582912 989476032   2% /\n";
        assert_eq!(parse_df(t), Some((12582912 * 1024, 1055762868 * 1024)));
        assert!(parse_df("rien").is_none());
    }
}
