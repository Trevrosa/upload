# upload service written with Rocket

## api reference

### PUT `/`
upload a file specified in the `file` field of the request's `multipart/form-data` body. 

the form must specify the file's name.

### PUT `/multi/<id>/<num>`
this endpoint allows a user to split up their upload into multiple requests.

the body should be specified the same as in the normal upload.

- `<id>` specifies the unique id for and must be a string.
- `<num>` specifies the request's order and must be a number. 

### PUT `/done/<id>/<name>`
combine the files uploaded with `<id>` to a file named `<name>`

- `<id>` must be a string.
- `<name>` must be a string.

## example usage (in javascript)
upload a file
```js
let auth = new Headers();
auth.set("token", token);
let form = new FormData();
form.set("file", file); // file being an instance of File

let resp = await fetch("/", {
  method: "PUT", headers: auth, body: form
});
```
