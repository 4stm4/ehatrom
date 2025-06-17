FROM rustlang/rust:nightly

# Устанавливаем рабочую директорию внутри контейнера
WORKDIR /ehatrom

# Добавляем необходимые компоненты для анализа и форматирования кода
RUN rustup component add rustfmt clippy
