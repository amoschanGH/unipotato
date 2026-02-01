use unipotato::{Request, Response, handler::html, get};

#[get("/")]
pub async fn index(_req: Request) -> Response {
    html(r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Unipotato CRUD App</title>
            <style>
                body { font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }
                h1 { color: #ff6b35; }
                .user-item { background: #f5f5f5; padding: 15px; margin: 10px 0; border-radius: 5px; }
                .user-item strong { color: #333; }
                button { background: #ff6b35; color: white; border: none; padding: 8px 15px; margin: 5px; cursor: pointer; border-radius: 3px; }
                button:hover { background: #ff5722; }
                .delete-btn { background: #dc3545; }
                .delete-btn:hover { background: #c82333; }
                input, button { margin: 5px 0; padding: 8px; }
                input { width: 200px; }
                form { margin: 20px 0; padding: 20px; background: #f9f9f9; border-radius: 5px; }
            </style>
        </head>
        <body>
            <h1>Unipotato CRUD Application</h1>
            <p>Complete CRUD operations with JSON file storage</p>
            
            <h2>Users</h2>
            <div id="users-list"></div>
            
            <h2>Create New User</h2>
            <form onsubmit="createUser(event)">
                <input type="text" id="name" placeholder="Name" required><br>
                <input type="email" id="email" placeholder="Email" required><br>
                <button type="submit">Create User</button>
            </form>

            <script>
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
                    document.getElementById('users-list').innerHTML = html;
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
