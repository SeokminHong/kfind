# Chocolatey 배포

Tagged release는 Windows x64 portable archive와 같은 checksum을 가리키는
`kfind.VERSION.nupkg`를 GitHub Release에 첨부한다. Chocolatey package는 archive를 package
directory에 풀고 `bin/kfind.exe`의 자동 shim을 사용한다.

템플릿만 검증하려면 PowerShell 7에서 다음 명령을 실행한다.

```powershell
pwsh scripts/test-chocolatey-package.ps1 -SkipPack
```

Windows에서는 Chocolatey package 생성까지 검증한다.

```powershell
pwsh scripts/test-chocolatey-package.ps1
```

Community Repository 게시 workflow는 GitHub Actions secret
`CHOCOLATEY_API_KEY`를 사용한다. Secret이 없으면 release workflow가 archive와 package를
만들고 설치 smoke test까지 실행하지만 remote push는 건너뛴다. Secret을 등록한 뒤
`Publish Chocolatey` workflow를 같은 release tag로 수동 실행하면 된다.

실제 게시 전에는 `kfind` package ID가 비어 있거나 계정에 할당됐는지 Chocolatey에서
확인해야 한다.
