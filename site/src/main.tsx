import './theme.css';
import './site.css';
import './content.css';
import './playground.css';
import './components/diagram.css';

import { createRoot } from 'react-dom/client';
import { RouterProvider } from 'react-router/dom';

import { router } from './app/router';

const root = document.querySelector('#root');

if (!(root instanceof HTMLElement)) {
  throw new TypeError('kfind documentation root element is missing');
}

createRoot(root).render(<RouterProvider router={router} />);
