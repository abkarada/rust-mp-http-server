/* 
    Anotompy of an HTTP message:
    +------------------------------+
    POST / HTTP/1.1, Meth  & vers. | --> Start Line
    +------------------------------+
    Host: developer.mozilla.org    |
    User-Agent: curl/8.6.0         |
    Accept: ./.                    | --> Headers
    Content-Type: application/json |
    Content-Length: 345            |
    +------------------------------+ -> Empty Line
    {                              |
     "data": "ABC123"              | --> Body 
    }                              |
    +------------------------------+


    1. A start line is Method / HTTP version
    2. An optional set of HTTP headers containing 
      metadata that describes the message.
    3. An empty line indicating the metadata of the 
        message is complete.
    4. An optional body containing data associated with
        the message.This might be POST data to send to
        server in a request, or some resource returned to
        the client in a response:
        Whether a message contains a body or not is determined
            by the start-line and HTTP headers.


    HTTP Request

    POST /users HTTP/1.1 -> start line
    Host: example.com | header
    Content-Type: application/x-www-form-urlencoded | -> header
    Content-Length: 49 | header
    // -> empty line
    name=FirstName+LastName&email=bsmth%40example.com -> body 

    start-line : request-line:
    <method> <request-target> <protocol>

    GET / HTTP/1.1\r\n
    Host: localhost:4221\r\n
    User-Agent: curl/8.20.0\r\n
    \r\n
    <body>

    Safe Method in terms of HTTP
    Safe Method simply a method that not changing the
    state of the server:
    An HTTP method is safe if it doesn't alter the
    state of the server.In other words, a method is sage
    it it leads to a aread-only operation.

    Idempotent Method in terms of HTTP
    A client can safely retry a request that uses
    an idempotent method.
    

    Request:

    GET /echo/abc HTTP/1.1\r\n
    Host: localhost:4221\r\n
    User-Agent: curl/7.64.1\r\n
    Accept: ./.\r\n
    \r\n

    Response:
    HTTP/1.1 200 OK\r\n
    Content-Type: text/plain\r\n
    Content-Length: 3\r\n\
    r\nabc




    User-Agent: <product> / <product-version> <comment>

    User-Agent: Mozilla/5.0 (<system-information>) <platform> (<platform-details>) <extensions>

    GET /home.html HTTP/1.1
    Host: developer.mozilla.org
    User-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10.9; rv:50.0) Gecko/20100101 Firefox/50.0
    Accept: text/html,application/xhtml+xml,application/xml;q=0.9,;q=0.8
    Accept-Language: en-US,en;q=0.5
    Accept-Encoding: gzip, deflate, br
    Referer: https://developer.mozilla.org/testpage.html
    Connection: keep-alive
    Upgrade-Insecure-Requests: 1
    If-Modified-Since: Mon, 18 Jul 2016 02:36:04 GMT
    If-None-Match: "c561c68d0ba92bbeb8b0fff2a9199f722e3a621a"
    Cache-Control: max-age=0

    */