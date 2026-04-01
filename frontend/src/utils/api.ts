import { isDemoMode, handleDemoRequest } from './demo';

export async function apiFetch(url: string, options: RequestInit = {}) {
  // Demo 模式重定向
  if (isDemoMode && url.startsWith('/api')) {
    console.log('[Demo] Intercepting fetch: ', url);
    return handleDemoRequest(url, options);
  }

  const token = localStorage.getItem('token');
  
  const headers = new Headers(options.headers || {});
  if (token && !headers.has('Authorization')) {
    headers.set('Authorization', `Bearer ${token}`);
  }

  const response = await fetch(url, {
    ...options,
    headers,
  });

  if (response.status === 401) {
    localStorage.removeItem('token');
    localStorage.removeItem('username');
    window.location.reload();
    // Return a never-resolving promise to stop further processing in the component
    return new Promise<Response>(() => {});
  }

  return response;
}
