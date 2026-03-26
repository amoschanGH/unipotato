use unipotato::{Request, Response, handler::html, get};

#[get("/")]
pub async fn index(_req: Request) -> Response {
    html(r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Unipotato CRUD App</title>
            <style>
                body { font-family: Arial, sans-serif; max-width: 900px; margin: 0 auto; padding: 20px; }
                h1 { color: #ec9191; }
                h2 { color: #333; border-bottom: 2px solid #ff6b35; padding-bottom: 5px; }
                h3 { color: #666; margin-top: 20px; }
                .user-item { background: #f5f5f5; padding: 15px; margin: 10px 0; border-radius: 5px; }
                .user-item strong { color: #333; }
                button { background: #ff6b35; color: white; border: none; padding: 8px 15px; margin: 5px; cursor: pointer; border-radius: 3px; }
                button:hover { background: #ff5722; }
                .delete-btn { background: #dc3545; }
                .delete-btn:hover { background: #c82333; }
                .test-btn { background: #28a745; }
                .test-btn:hover { background: #218838; }
                input, button { margin: 5px 0; padding: 8px; }
                input { width: 200px; }
                form { margin: 20px 0; padding: 20px; background: #f9f9f9; border-radius: 5px; }
                .test-section { background: #e8f4f8; padding: 20px; border-radius: 5px; margin: 20px 0; }
                .result { background: #fff; padding: 15px; margin: 10px 0; border-radius: 5px; border: 1px solid #ddd; font-family: monospace; white-space: pre-wrap; }
                .param-input { width: 80px; margin-right: 10px; }
                .test-row { margin: 10px 0; display: flex; align-items: center; flex-wrap: wrap; gap: 10px; }
                .endpoint { color: #007bff; font-weight: bold; min-width: 350px; }
                code { background: #f4f4f4; padding: 2px 6px; border-radius: 3px; }
            </style>
        </head>
        <body>
            <h1>Unipotato CRUD Application</h1>
            <p>Complete CRUD operations with JSON file storage + Path Parameter Examples</p>
            
            <h2>Users</h2>
            <div id="users-list"></div>
            
            <h2>Create New User</h2>
            <form onsubmit="createUser(event)">
                <input type="text" id="name" placeholder="Name" required><br>
                <input type="email" id="email" placeholder="Email" required><br>
                <button type="submit">Create User</button>
            </form>

            <h2>Path Parameter Tests</h2>
            <div class="test-section">
                <h3>Two Parameters: <code>/api/users/&lt;user_id&gt;/posts/&lt;post_id&gt;</code></h3>
                <div class="test-row">
                    <span class="endpoint">GET /api/users/{user_id}/posts/{post_id}</span>
                    <input type="number" id="user_id_1" class="param-input" placeholder="user_id" value="1">
                    <input type="number" id="post_id_1" class="param-input" placeholder="post_id" value="42">
                    <button class="test-btn" onclick="testUserPost()">Test</button>
                </div>
                <div id="result-user-post" class="result" style="display:none;"></div>

                <h3>Three Parameters: <code>/api/users/&lt;user_id&gt;/posts/&lt;post_id&gt;/comments/&lt;comment_id&gt;</code></h3>
                <div class="test-row">
                    <span class="endpoint">GET /api/users/{user_id}/posts/{post_id}/comments/{comment_id}</span>
                    <input type="number" id="user_id_2" class="param-input" placeholder="user_id" value="1">
                    <input type="number" id="post_id_2" class="param-input" placeholder="post_id" value="42">
                    <input type="number" id="comment_id" class="param-input" placeholder="comment_id" value="5">
                    <button class="test-btn" onclick="testPostComment()">Test</button>
                </div>
                <div id="result-post-comment" class="result" style="display:none;"></div>

                <h3>String Parameters: <code>/api/categories/&lt;category&gt;/products/&lt;product_slug&gt;</code></h3>
                <div class="test-row">
                    <span class="endpoint">GET /api/categories/{category}/products/{slug}</span>
                    <input type="text" id="category" class="param-input" placeholder="category" value="electronics" style="width:120px;">
                    <input type="text" id="product_slug" class="param-input" placeholder="product_slug" value="laptop-pro" style="width:120px;">
                    <button class="test-btn" onclick="testCategoryProduct()">Test</button>
                </div>
                <div id="result-category-product" class="result" style="display:none;"></div>

                <h3>Async Test: <code>/api/slow</code> (3 second delay)</h3>
                <div class="test-row">
                    <span class="endpoint">GET /api/slow</span>
                    <button class="test-btn" onclick="testSlow()">Test Async</button>
                    <span id="slow-status"></span>
                </div>
                <div id="result-slow" class="result" style="display:none;"></div>
            </div>

            <script>
                // ============ User CRUD Functions ============
                async function loadUsers() {
                    const res = await fetch('/api/users');
                    const users = await res.json();
                    const html = users.map(u => `
                        <div class="user-item">
                            <strong>${u.name}</strong> - ${u.email}
                            <div>
                                <button onclick="editUser(${u.id}, '${u.name}', '${u.email}')">Edit</button>
                                <button class="delete-btn" onclick="deleteUser(${u.id})">Delete</button>
                            </div>
                        </div>
                    `).join('');
                    document.getElementById('users-list').innerHTML = html || '<p>No users yet.</p>';
                }

                async function createUser(e) {
                    e.preventDefault();
                    const name = document.getElementById('name').value;
                    const email = document.getElementById('email').value;
                    await fetch('/api/users', {
                        method: 'POST',
                        headers: {'Content-Type': 'application/json'},
                        body: JSON.stringify({name, email})
                    });
                    document.getElementById('name').value = '';
                    document.getElementById('email').value = '';
                    loadUsers();
                }

                async function editUser(id, name, email) {
                    const newName = prompt('Enter new name:', name);
                    const newEmail = prompt('Enter new email:', email);
                    if (newName && newEmail) {
                        await fetch('/api/users/' + id, {
                            method: 'PUT',
                            headers: {'Content-Type': 'application/json'},
                            body: JSON.stringify({name: newName, email: newEmail})
                        });
                        loadUsers();
                    }
                }

                async function deleteUser(id) {
                    if (confirm('Are you sure you want to delete this user?')) {
                        await fetch('/api/users/' + id, {method: 'DELETE'});
                        loadUsers();
                    }
                }

                // ============ Path Parameter Test Functions ============
                async function testUserPost() {
                    const userId = document.getElementById('user_id_1').value;
                    const postId = document.getElementById('post_id_1').value;
                    const url = `/api/users/${userId}/posts/${postId}`;
                    
                    const res = await fetch(url);
                    const data = await res.json();
                    
                    const resultEl = document.getElementById('result-user-post');
                    resultEl.style.display = 'block';
                    resultEl.innerHTML = `<strong>URL:</strong> ${url}\n<strong>Response:</strong>\n${JSON.stringify(data, null, 2)}`;
                }

                async function testPostComment() {
                    const userId = document.getElementById('user_id_2').value;
                    const postId = document.getElementById('post_id_2').value;
                    const commentId = document.getElementById('comment_id').value;
                    const url = `/api/users/${userId}/posts/${postId}/comments/${commentId}`;
                    
                    const res = await fetch(url);
                    const data = await res.json();
                    
                    const resultEl = document.getElementById('result-post-comment');
                    resultEl.style.display = 'block';
                    resultEl.innerHTML = `<strong>URL:</strong> ${url}\n<strong>Response:</strong>\n${JSON.stringify(data, null, 2)}`;
                }

                async function testCategoryProduct() {
                    const category = document.getElementById('category').value;
                    const slug = document.getElementById('product_slug').value;
                    const url = `/api/categories/${category}/products/${slug}`;
                    
                    const res = await fetch(url);
                    const data = await res.json();
                    
                    const resultEl = document.getElementById('result-category-product');
                    resultEl.style.display = 'block';
                    resultEl.innerHTML = `<strong>URL:</strong> ${url}\n<strong>Response:</strong>\n${JSON.stringify(data, null, 2)}`;
                }

                async function testSlow() {
                    const statusEl = document.getElementById('slow-status');
                    const resultEl = document.getElementById('result-slow');
                    
                    statusEl.textContent = '⏳ Waiting (3 seconds)...';
                    const start = Date.now();
                    
                    const res = await fetch('/api/slow');
                    const data = await res.json();
                    
                    const elapsed = ((Date.now() - start) / 1000).toFixed(2);
                    statusEl.textContent = `✅ Completed in ${elapsed}s`;
                    
                    resultEl.style.display = 'block';
                    resultEl.innerHTML = `<strong>Response:</strong>\n${JSON.stringify(data, null, 2)}`;
                }

                // Load users on page load
                loadUsers();
            </script>
        </body>
        </html>
    "#)
}

#[get("/about")]
pub async fn about(_req: Request) -> Response {
    html("<h1>About</h1><p>This is a simple CRUD app with JSON storage</p><a href='/'>Back</a>")
}
