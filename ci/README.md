Le jeton de Harness ne peut pas écrire dans `.github/workflows`. Pour activer la CI :

    mkdir -p .github/workflows && git mv ci/ci.yml .github/workflows/ci.yml

puis commit et push depuis ta machine.
