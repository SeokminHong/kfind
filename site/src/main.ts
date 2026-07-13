import './theme.css';

const status = document.querySelector<HTMLElement>('#boot-status');

if (status !== null) {
  status.textContent = '사이트 빌드 준비가 완료되었습니다.';
}
