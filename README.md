# upload service

## api reference

### PUT `/`
upload a file specified in the `file` field of the request's `multipart/form-data` body. 

*the form must specify the file's name.*

*the request must have a header named `token` with the set token*

### PUT `/multi/<id>/<num>`
this endpoint allows a user to split up their upload into multiple requests.

*the request body should be specified the same as in the normal upload.*

*the request must have a header named `token` with the set token*

- `<id>` specifies the unique id for and must be a string.
- `<num>` specifies the request's order and must be a number. 

### GET `/done/<id>/<name>/<total>`
combine the files uploaded with `<id>` to the final file named `<name>`

- `<id>` must be a string.
- `<name>` must be a string.
- `<total>` is the expected total chunks to be combined and must be a number.

***this is a special endpoint that returns a stream of [server sent events](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events)***

the event `id`s that can be returned are (every event is a `message`):

- `idnotfound` - data: error message
- `missingchunks` - data: error message
- `duplicate` - data: error message
- `servererror` - data: error message
- `progress` - data: total number of files combined
- `done` - data: url of upload

*the client should close the connection (or stop reconnecting) on receiving any `event` with `id` not `progress`.*

an `event` without an `id` is a "[heartbeat event](https://api.rocket.rs/v0.5/rocket/response/stream/struct.EventStream#heartbeat)" (an empty comment) meant to keep the connection alive.

## example usage (javascript)
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

### an example static web app that uses all endpoints can be found [here](https://github.com/Trevrosa/upload/tree/main/web).