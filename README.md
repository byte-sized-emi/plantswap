# plantswap

plantswap is (or wants to be, currently WIP) a platform for selling, buying, and most
importantly trading plants/cuttings with other people, using the help of
plant recognition AI's like [plantnet.org](https://plantnet.org/).

## TODO:
 - [ ] Make templates borrow data instead of own it
 - [ ] Logout button
 - [x] 404 page
 - [ ] create account functionality
 - [ ] Display errors in create listing page directly in form (+ don't reset form data when it does)
 - [ ] fine grained access control

## Docker tags

 - `dev-{{number}}`: Built from main branch, increases after each run. For testing.
 - `dev-latest`: Latest `dev-*` build
 - `pr-{{number}}`: Built when a pull request is marked as ready to review
 - `release-v{{version}}`: Not used currently, will be used with tags to release versions
 - `latest`: Will be used together with the release tags

## Testing

Install `cargo-nextest`, for example through `cargo binstall cargo-nextest --secure`.

Then, run `cargo nextest run`.

## Dev

### Test file upload with cURL

```bash
curl --form file='@plant-desk.jpg' http://localhost:3000/api/v1/image -v
```
