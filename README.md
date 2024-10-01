
[Database.md](Database.md)

## TODO:
 - [ ] Make templates borrow data instead of own it
 - [ ] Logout button
 - [ ] 404 page

## Plant identification sites

### Offline

https://plantnet.org/en/2022/10/18/plntnet-offline-embedded-identify-plants-anywhere-without-connection/

### Online

https://www.kindwise.com/plant-id

https://plantnet.org/en/

# Dev

## Test file upload with cURL

```bash
curl --form file='@plant-desk.jpg' http://localhost:3000/api/v1/image -v
```
