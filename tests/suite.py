import requests

LOCATION: str = "http://127.0.0.1:3000/"

def get(uri: str):
    response = requests.get(LOCATION+uri)
    print(f"HTTP/1.1 {response.status_code} {response.reason}")

    for name, value in response.headers.items():
        print(f"{name}: {value}")

    print()
    print(response.text)

def post(uri: str, data: str = ""):
    response = requests.post(LOCATION+uri, data)
    print(f"HTTP/1.1 {response.status_code} {response.reason}")

    for name, value in response.headers.items():
        print(f"{name}: {value}")

    print()
    print(response.text)

while True:
    parts = input(">").split(" ", 2)
    parts += [""] * (3 - len(parts))

    m, l, d = parts

    match m:
        case "get":
            get(l)
        case "post":
            post(l, d)