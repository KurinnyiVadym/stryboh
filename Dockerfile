FROM arm32v7/rust:1.77

WORKDIR /usr/src/stryboh
COPY . .

RUN cargo install --path .

CMD ["stryboh"]