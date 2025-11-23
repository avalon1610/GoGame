const fs = require('fs');
const path = require('path');

try {
    const tauriConfPath = path.join('src-tauri', 'tauri.conf.json');
    const packageJsonPath = 'package.json';
    const cargoTomlPath = path.join('src-tauri', 'Cargo.toml');

    const tauriConf = JSON.parse(fs.readFileSync(tauriConfPath, 'utf8'));
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
    const cargoToml = fs.readFileSync(cargoTomlPath, 'utf8');

    const tauriVersion = tauriConf.package.version;
    const packageVersion = packageJson.version;
    
    // Simple regex to find version in [package] section of Cargo.toml
    // This assumes version is one of the first keys in [package]
    const cargoVersionMatch = cargoToml.match(/^version\s*=\s*"([^"]+)"/m);
    const cargoVersion = cargoVersionMatch ? cargoVersionMatch[1] : 'unknown';

    console.log(`Tauri version: ${tauriVersion}`);
    console.log(`Package version: ${packageVersion}`);
    console.log(`Cargo version: ${cargoVersion}`);

    let hasError = false;

    if (tauriVersion !== packageVersion) {
        console.error(`❌ Error: package.json version (${packageVersion}) does not match tauri.conf.json version (${tauriVersion})`);
        hasError = true;
    }

    if (tauriVersion !== cargoVersion) {
        console.error(`❌ Error: Cargo.toml version (${cargoVersion}) does not match tauri.conf.json version (${tauriVersion})`);
        hasError = true;
    }

    if (hasError) {
        process.exit(1);
    }

    console.log('✅ All versions match.');

} catch (error) {
    console.error('Error reading files:', error);
    process.exit(1);
}
