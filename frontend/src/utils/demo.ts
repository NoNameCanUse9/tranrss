import initSqlJs, { Database } from 'sql.js';

let db: Database | null = null;
let SQL: any = null;

export const isDemoMode = import.meta.env.VITE_DEMO_MODE === 'true' || 
                          window.location.hostname.includes('github.io') ||
                          window.location.hostname.includes('demo');

export async function initDemo() {
  if (!isDemoMode) return;
  if (db) return;

  try {
    SQL = await initSqlJs({
      locateFile: file => `${import.meta.env.BASE_URL}data/${file}`
    });
    const response = await fetch(`${import.meta.env.BASE_URL}data/demo.db`);
    const buffer = await response.arrayBuffer();
    db = new SQL.Database(new Uint8Array(buffer));
    console.log('Demo database loaded successfully');
  } catch (e) {
    console.error('Failed to load demo database', e);
  }
}

function exec(sql: string, params: any = {}) {
  if (!db) return [];
  const stmt = db.prepare(sql);
  stmt.bind(params);
  const rows = [];
  while (stmt.step()) {
    rows.push(stmt.getAsObject());
  }
  stmt.free();
  return rows;
}

export async function handleDemoRequest(url: string, options: RequestInit = {}) {
  if (!db) await initDemo();
  
  const path = url.split('?')[0];
  const searchParams = new URL(url, window.location.origin).searchParams;

  // 1. Feeds
  if (path.endsWith('/api/feeds')) {
    const feeds = exec(`
      SELECT f.*, COALESCE(s.custom_title, f.title) as title, s.id as sub_id,
      (SELECT COUNT(*) FROM articles a WHERE a.feed_id = f.id AND a.is_read = 0) as unreadCount,
      (SELECT COUNT(*) FROM articles a WHERE a.feed_id = f.id AND a.is_starred = 1) as starredCount,
      COALESCE(fl.title, '未分类') as category
      FROM feeds f
      JOIN subscriptions s ON f.id = s.feed_id
      LEFT JOIN folders fl ON s.folder_id = fl.id
    `);
    return new Response(JSON.stringify(feeds), { status: 200 });
  }

  // 2. Articles List
  if (path.endsWith('/api/articles')) {
    let sql = `SELECT a.*, f.title as feedTitle FROM articles a JOIN feeds f ON a.feed_id = f.id WHERE 1=1`;
    const params: any = {};
    
    const feedId = searchParams.get('feed_id');
    if (feedId) {
      sql += ` AND a.feed_id = :feed_id`;
      params[':feed_id'] = parseInt(feedId);
    }
    
    const isRead = searchParams.get('is_read');
    if (isRead !== null) {
      sql += ` AND a.is_read = :is_read`;
      params[':is_read'] = isRead === 'true' ? 1 : 0;
    }

    const isStarred = searchParams.get('is_starred');
    if (isStarred !== null) {
      sql += ` AND a.is_starred = :is_starred`;
      params[':is_starred'] = isStarred === 'true' ? 1 : 0;
    }

    sql += ` ORDER BY a.published_at DESC LIMIT 100`;
    const articles = exec(sql, params);
    return new Response(JSON.stringify(articles), { status: 200 });
  }

  // 3. Article Detail
  const articleMatch = path.match(/\/api\/articles\/(\d+)$/);
  if (articleMatch) {
    const id = parseInt(articleMatch[1]);
    const detail = exec(`SELECT * FROM articles WHERE id = :id`, { ':id': id })[0];
    const blocks = exec(`SELECT * FROM article_blocks WHERE article_id = :id ORDER BY block_index ASC`, { ':id': id });
    const is_need_translated = !!exec(`SELECT need_translate FROM subscriptions WHERE feed_id = :fid`, { ':fid': detail.feed_id })[0]?.need_translate;

    return new Response(JSON.stringify({
      detail,
      blocks,
      content: "", // Content stitching usually done in backend, here we just return raw for simplicity or implement stitch
      is_need_translated
    }), { status: 200 });
  }

  // 4. User settings (mocked for demo)
  if (path.endsWith('/api/user/setting')) {
    return new Response(JSON.stringify({
      translate_api_id: null,
      summary_api_id: null,
      default_api_id: null,
      greader_api: false,
      api_proxy: false,
      app_mode: false,
      log_num_limit: 300,
      custom_trans_style: "display: block; font-style: italic; opacity: 0.6;"
    }), { status: 200 });
  }

  // 5. Jobs list (empty for demo)
  if (path.endsWith('/api/jobs')) {
    return new Response(JSON.stringify([]), { status: 200 });
  }

  // Default fallback for demo (success or empty)
  return new Response(JSON.stringify({ status: "demo_success" }), { status: 200 });
}
