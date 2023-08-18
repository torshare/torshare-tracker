var net = require("net");

const requestBody = 'This is the request body content';
const httpRequest = [
  "GET /scrape HTTP/1.1",
  "User-Agent: Node.js HTTP Client",
  "Connection: keep-alive",
  'Content-Length: ' + Buffer.byteLength(requestBody),
  "", // Empty line to indicate end of headers
].join("\r\n");

var client = new net.Socket();

function sendRequest() {
  client.write(`${httpRequest}`);
  client.write('\r\n');
  client.write(requestBody);
}

client.connect(3000, "127.0.0.1", function () {
  console.log("Connected");
  sendRequest();
});

var timer = 0;

client.on("data", function (data) {
    console.log("Received: " + data);
    // client.destroy(); // kill client after server's response
    timer = setTimeout(sendRequest, 10000);
});

client.on("close", function () {
  clearTimeout(timer);
  console.log("Connection closed");
});

client.on("error", function (err) {
  console.error(err);
});