# Builds the documentation site. Diagrams are rendered during `mdbook build`.
FROM rust:1-slim AS build
WORKDIR /src
RUN cargo install mdbook --version 0.5.4 --locked
COPY . .
RUN cargo install --path docs/mdbook-drumlin --locked
RUN mdbook build docs

FROM nginx:1.27-alpine
COPY --from=build /src/docs/book /usr/share/nginx/html
EXPOSE 80
