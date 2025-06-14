# LLM Text Translator - Desktop Application (Updated)

## Описание проекта

Десктопное приложение для быстрого перевода выделенного текста через LLM (GPT-4.1-nano). Пользователь выделяет текст в любом приложении, нажимает горячую клавишу, и текст автоматически переводится и заменяется на переведенный вариант.

## Основные функции

- Перевод выделенного текста по горячей клавише
- Автоматическая замена выделенного текста на переведенный
- Трей-приложение с настройками
- **Предустановленные промпты для разных стилей** _(новое)_:
  - Twitter стиль (краткий, casual)
  - Официальный стиль (formal, business)
  - Академический стиль (scholarly)
  - Творческий стиль (artistic)
  - Пользовательские промпты
- Быстрое переключение между промптами через горячие клавиши или меню
- Выбор модели LLM
- Управление API ключом
- **Защита от edge cases и конфликтов** _(новое)_

## Технологический стек

### Основной стек - Rust + Tauri

- **Язык**: Rust (backend) + HTML/CSS/JS (frontend)
- **Фреймворк**: Tauri 2.0
- **Горячие клавиши**: `global-hotkey` v0.5+
- **Буфер обмена**: `arboard` v3.3+
- **HTTP клиент**: `reqwest` v0.11+ с поддержкой retry
- **Асинхронность**: `tokio` v1.35+
- **Сериализация**: `serde` v1.0+ с `serde_json`
- **Конфигурация**: `toml` v0.8+
- **Логирование**: `tracing` + `tracing-subscriber`
- **Тестирование**: встроенный test framework + `mockall` для моков
- **Валидация**: `validator` v0.16+ _(новое)_

### UI компоненты

- **Frontend**: Vanilla JS + современный CSS (без фреймворков для минимального размера)
- **Иконки**: Tauri Icon для трея
- **Уведомления**: нативные системные уведомления через Tauri

## Архитектура приложения

### Модульная структура

```
src/
├── main.rs                 # Entry point
├── config/
│   ├── mod.rs             # Конфигурация приложения
│   └── settings.rs        # Структуры настроек
├── hotkeys/
│   ├── mod.rs             # Управление горячими клавишами
│   ├── listener.rs        # Слушатель событий
│   ├── handlers.rs        # Обработчики горячих клавиш
│   └── validator.rs       # Валидатор конфликтов *(новое)*
├── clipboard/
│   ├── mod.rs             # Работа с буфером обмена
│   └── manager.rs         # Менеджер буфера
├── llm/
│   ├── mod.rs             # Клиент для LLM
│   ├── client.rs          # HTTP клиент для OpenAI
│   └── models.rs          # Структуры данных API
├── validation/             # *(новый модуль)*
│   ├── mod.rs             # Общая валидация
│   ├── text.rs            # Валидация текста
│   └── limits.rs          # Проверка лимитов
├── tray/
│   ├── mod.rs             # Трей функциональность
│   └── menu.rs            # Контекстное меню
└── tests/
    ├── unit/              # Unit тесты
    └── integration/       # Интеграционные тесты
```

### Поток данных (обновленный)

1. **HotkeyListener** перехватывает глобальную горячую клавишу
2. **HotkeyValidator** проверяет отсутствие конфликтов _(новое)_
3. **ClipboardManager** копирует выделенный текст (симулирует Ctrl+C)
4. **TextValidator** проверяет валидность текста _(новое)_
5. **LLMClient** отправляет текст на перевод
6. **ClipboardManager** заменяет текст в буфере
7. **ClipboardManager** вставляет переведенный текст (симулирует Ctrl+V)

## Детальный план разработки

### Фаза 0: Подготовка (1 день)

- [x] Инициализация Rust проекта с Cargo
- [x] Настройка Tauri 2.0
- [x] Создание базовой структуры директорий
- [ ] Настройка CI/CD (GitHub Actions)
- [ ] Настройка pre-commit hooks для форматирования и линтинга

### Фаза 1: Core функциональность (4-5 дней)

#### 1.1 Модуль конфигурации

- [x] Создать структуры для настроек (Config, ApiSettings, HotkeySettings)
- [x] Реализовать загрузку/сохранение в TOML
- [x] Добавить валидацию настроек
- [x] **Добавить лимиты и правила валидации** _(новое)_
- [x] **Тесты**:
  - Unit: сериализация/десериализация
  - Unit: валидация невалидных конфигов
  - Unit: значения по умолчанию
  - Unit: проверка лимитов _(новое)_

#### 1.2 Модуль работы с буфером обмена

- [x] Обертка над `arboard` для кроссплатформенности
- [x] Методы get_text(), set_text()
- [x] Обработка ошибок и таймаутов
- [x] **Сохранение и восстановление оригинального буфера** _(новое)_
- [x] **Детекция пустого выделения** _(новое)_
- [x] **Тесты**:
  - Unit: mock тесты для clipboard операций
  - Unit: обработка пустого буфера _(новое)_
  - Integration: реальное копирование/вставка

#### 1.3 Модуль горячих клавиш

- [x] Интеграция `global-hotkey`
- [x] Регистрация/отмена регистрации комбинаций
- [x] **Детекция системных горячих клавиш** _(новое)_
- [x] **Проверка конфликтов с другими приложениями** _(новое)_
- [x] **Fallback механизм при конфликте** _(новое)_
- [x] **Тесты**:
  - Unit: парсинг строк горячих клавиш
  - Unit: валидация комбинаций
  - Unit: детекция конфликтов _(новое)_
  - Integration: симуляция нажатий

#### 1.4 Модуль валидации _(новый)_

- [x] **Валидатор текста**:
  - Проверка на пустую строку
  - Проверка максимальной длины
  - Проверка на только пробельные символы
  - Детекция бинарных данных
- [x] **Валидатор лимитов**:
  - Rate limiting (запросов в минуту)
  - Дневные лимиты
  - Размер очереди запросов
- [x] **Тесты**:
  - Unit: все виды невалидных текстов
  - Unit: граничные значения
  - Unit: Unicode edge cases

### Фаза 2: LLM интеграция (2-3 дня)

#### 2.1 HTTP клиент для OpenAI

- [x] Структуры для API запросов/ответов
- [x] Реализация клиента с retry логикой
- [x] Обработка ошибок API
- [x] Поддержка отмены запросов
- [x] **Обработка слишком больших запросов** _(новое)_
- [x] **Разбиение текста на чанки при необходимости** _(новое)_
- [x] **Тесты**:
  - Unit: сериализация запросов
  - Unit: парсинг ответов
  - Unit: retry логика с моками
  - Unit: обработка больших текстов _(новое)_
  - Integration: тест с реальным API (опционально)

#### 2.2 Менеджер переводов

- [x] Очередь запросов
- [x] Кеширование последних переводов
- [x] **Дедупликация запросов** _(новое)_
- [x] **Приоритизация коротких текстов** _(новое)_
- [x] **Тесты**:
  - Unit: управление очередью
  - Unit: работа кеша
  - Unit: дедупликация _(новое)_
  - Unit: обработка очереди при лимитах _(новое)_

### Фаза 3: UI и трей (3-4 дня)

#### 3.1 Системный трей

- [x] Иконка с контекстным меню
- [x] Индикация состояния (активно/неактивно/загрузка)
- [x] Быстрый доступ к настройкам
- [x] **Индикация ошибок и конфликтов** _(новое)_
- [x] **Тесты**:
  - Manual: проверка отображения на разных ОС

#### 3.2 Окно настроек

- [x] HTML/CSS интерфейс
- [x] Формы для всех настроек
- [x] Валидация на стороне UI
- [x] **Интерфейс для разрешения конфликтов горячих клавиш** _(новое)_
- [x] **Визуализация текущих лимитов** _(новое)_
- [x] **Тесты**:
  - Unit: валидация форм
  - Unit: проверка лимитов в UI _(новое)_
  - E2E: сценарии изменения настроек

### Фаза 4: Интеграция и полировка (3-4 дня)

#### 4.1 Основной workflow

- [ ] Связать все модули вместе
- [ ] Реализовать полный цикл перевода
- [ ] Добавить логирование всех этапов
- [ ] **Graceful обработка всех edge cases** _(новое)_
- [ ] **Тесты**:
  - Integration: полный сценарий перевода
  - Integration: обработка ошибок на каждом этапе
  - Integration: edge cases (пустой текст, огромный текст) _(новое)_

#### 4.2 Улучшения UX

- [ ] Звуковые уведомления
- [ ] Визуальные индикаторы процесса
- [ ] История последних переводов
- [ ] Статистика использования
- [ ] **Уведомления об ошибках с действиями** _(новое)_

### Фаза 5: Подготовка к релизу (1-2 дня)

- [ ] Сборка для всех платформ
- [ ] Подписание кода (Windows/macOS)
- [ ] Создание инсталляторов
- [ ] Написание документации пользователя
- [ ] Финальное тестирование на чистых системах

## Контроль качества кода _(новый раздел)_

### Проверки после каждой фазы

После завершения каждой фазы разработки необходимо:

1. **Код-ревью всех модулей:**

   - [ ] Проверка логических ошибок
   - [ ] Поиск потенциальных race conditions
   - [ ] Проверка обработки всех ошибок
   - [ ] Валидация edge cases

2. **Запуск всех тестов:**

   ```bash
   cargo test --all-features
   cargo clippy -- -D warnings
   cargo fmt --check
   ```

3. **Проверка безопасности:**

   - [ ] Отсутствие unwrap() в production коде
   - [ ] Правильная обработка sensitive данных (API ключи)
   - [ ] Проверка на memory leaks

4. **Интеграционное тестирование:**
   - [ ] Тест полного workflow перевода
   - [ ] Проверка работы с разными размерами текста
   - [ ] Тестирование всех error paths

## Защита от Edge Cases _(новый раздел)_

### Обработка пустого выделения

```rust
pub enum SelectionError {
    NoSelection,
    EmptySelection,
    OnlyWhitespace,
    ClipboardTimeout,
}

impl ClipboardManager {
    pub async fn get_selection(&mut self) -> Result<String, SelectionError> {
        let original = self.clipboard.get_text().ok();

        // Симулируем Ctrl+C
        simulate_copy().await?;

        // Ждем изменения с таймаутом
        let deadline = Instant::now() + Duration::from_millis(500);

        loop {
            if let Ok(current) = self.clipboard.get_text() {
                if Some(&current) != original.as_ref() && !current.is_empty() {
                    // Проверяем что это не только пробелы
                    if current.trim().is_empty() {
                        return Err(SelectionError::OnlyWhitespace);
                    }
                    return Ok(current);
                }
            }

            if Instant::now() > deadline {
                return Err(SelectionError::ClipboardTimeout);
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}
```

### Обработка больших текстов

```rust
pub struct TextValidator {
    max_length: usize,
    max_tokens_estimate: usize,
}

impl TextValidator {
    pub fn validate(&self, text: &str) -> Result<ValidationResult> {
        // Проверка длины
        if text.len() > self.max_length {
            return Ok(ValidationResult::TooLong {
                length: text.len(),
                max: self.max_length,
            });
        }

        // Оценка токенов (примерно 4 символа = 1 токен)
        let estimated_tokens = text.len() / 4;
        if estimated_tokens > self.max_tokens_estimate {
            return Ok(ValidationResult::TooManyTokens {
                estimated: estimated_tokens,
                max: self.max_tokens_estimate,
            });
        }

        // Проверка на бинарные данные
        if text.chars().any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t') {
            return Ok(ValidationResult::ContainsBinary);
        }

        Ok(ValidationResult::Valid)
    }
}

pub enum ValidationResult {
    Valid,
    TooLong { length: usize, max: usize },
    TooManyTokens { estimated: usize, max: usize },
    ContainsBinary,
}
```

### Защита от конфликтов горячих клавиш

```rust
pub struct HotkeyValidator {
    known_system_hotkeys: HashMap<Platform, Vec<KeyCombo>>,
}

impl HotkeyValidator {
    pub fn new() -> Self {
        let mut validator = Self {
            known_system_hotkeys: HashMap::new(),
        };

        // Windows
        validator.known_system_hotkeys.insert(Platform::Windows, vec![
            KeyCombo::new(&[Key::Alt, Key::Tab]),
            KeyCombo::new(&[Key::Alt, Key::F4]),
            KeyCombo::new(&[Key::Control, Key::Alt, Key::Delete]),
            KeyCombo::new(&[Key::Windows, Key::L]),
            // ... другие системные комбинации
        ]);

        // macOS
        validator.known_system_hotkeys.insert(Platform::MacOS, vec![
            KeyCombo::new(&[Key::Command, Key::Q]),
            KeyCombo::new(&[Key::Command, Key::W]),
            KeyCombo::new(&[Key::Command, Key::Space]),
            // ...
        ]);

        validator
    }

    pub fn validate(&self, combo: &KeyCombo) -> ValidationResult {
        let platform = Platform::current();

        // Проверяем системные хоткеи
        if let Some(system_hotkeys) = self.known_system_hotkeys.get(&platform) {
            if system_hotkeys.contains(combo) {
                return ValidationResult::SystemConflict;
            }
        }

        // Проверяем уже зарегистрированные
        if self.is_already_registered(combo) {
            return ValidationResult::AlreadyRegistered;
        }

        // Проверяем минимальную сложность (минимум 2 клавиши)
        if combo.keys.len() < 2 {
            return ValidationResult::TooSimple;
        }

        ValidationResult::Valid
    }

    pub fn suggest_alternative(&self, combo: &KeyCombo) -> Option<KeyCombo> {
        // Предлагаем альтернативы
        let alternatives = vec![
            KeyCombo::new(&[Key::Control, Key::Shift, Key::T]),
            KeyCombo::new(&[Key::Control, Key::Alt, Key::T]),
            KeyCombo::new(&[Key::Alt, Key::Shift, Key::T]),
        ];

        alternatives.into_iter()
            .find(|alt| self.validate(alt) == ValidationResult::Valid)
    }
}
```

### Rate Limiting

```rust
pub struct RateLimiter {
    requests: VecDeque<Instant>,
    max_per_minute: usize,
    max_per_day: usize,
    daily_count: usize,
    last_reset: DateTime<Utc>,
}

impl RateLimiter {
    pub fn check_and_update(&mut self) -> Result<(), RateLimitError> {
        let now = Instant::now();
        let today = Utc::now().date();

        // Сброс дневного счетчика
        if today != self.last_reset.date() {
            self.daily_count = 0;
            self.last_reset = Utc::now();
        }

        // Удаляем старые запросы (старше минуты)
        self.requests.retain(|&req| now.duration_since(req) < Duration::from_secs(60));

        // Проверяем лимит в минуту
        if self.requests.len() >= self.max_per_minute {
            let wait_time = Duration::from_secs(60) - now.duration_since(self.requests[0]);
            return Err(RateLimitError::MinuteLimit { wait_time });
        }

        // Проверяем дневной лимит
        if self.daily_count >= self.max_per_day {
            return Err(RateLimitError::DailyLimit);
        }

        // Обновляем счетчики
        self.requests.push_back(now);
        self.daily_count += 1;

        Ok(())
    }
}
```

## Дефолтные настройки (обновленные)

```toml
[general]
auto_start = true
show_notifications = true

[hotkey]
translate = "Ctrl+Shift+T"
cancel = "Escape"
# Альтернативные комбинации при конфликте
alternatives = ["Ctrl+Alt+T", "Alt+Shift+T"]

[api]
endpoint = "https://api.openai.com/v1/chat/completions"
model = "gpt-4.1-nano"
temperature = 0.3
max_retries = 3
timeout_seconds = 30

[prompt]
# Текущий активный промпт
active_preset = "general"

# Предустановленные промпты
[prompt.presets.general]
name = "Общий перевод"
system = "Переведи этот текст на грамотный английский язык, сохраняя смысл изначального текста. Верни только переведенный текст без каких-либо дополнений от себя."

[prompt.presets.twitter]
name = "Twitter стиль"
system = "Translate this text to English in a casual Twitter style. Keep it concise, use common abbreviations where appropriate, and maintain the original tone. Return only the translated text."

[prompt.presets.formal]
name = "Официальный стиль"
system = "Translate this text into formal, professional English suitable for business correspondence. Maintain proper grammar and formal vocabulary. Return only the translated text."

[prompt.presets.academic]
name = "Академический стиль"
system = "Translate this text into academic English with precise terminology and formal structure. Ensure clarity and scholarly tone. Return only the translated text."

[prompt.presets.creative]
name = "Творческий стиль"
system = "Translate this text into English with creative flair, maintaining the emotional impact and artistic expression of the original. Return only the translated text."

# Пользовательские промпты
[prompt.custom]
# Здесь пользователь может добавить свои промпты через UI

[limits]
max_text_length = 5000          # Максимальная длина текста в символах
max_tokens_estimate = 1250      # Примерная оценка токенов
min_text_length = 1             # Минимальная длина (защита от пустого)
requests_per_minute = 30        # Лимит запросов в минуту
requests_per_day = 500          # Лимит запросов в день
clipboard_timeout_ms = 500      # Таймаут ожидания буфера обмена

[validation]
allow_only_whitespace = false   # Разрешать перевод только пробелов
detect_binary_data = true       # Детектировать бинарные данные
trim_before_validate = true     # Обрезать пробелы перед проверкой

[behavior]
preserve_clipboard = true       # Сохранять оригинальный буфер
show_length_warning = true      # Предупреждать о длинном тексте
auto_split_long_text = false    # Автоматически разбивать длинный текст
```

## Обработка ошибок (расширенная)

### Стратегия fallback

1. **Горячие клавиши не работают** → уведомление + предложение альтернативной комбинации
2. **Конфликт горячих клавиш** → автоматический выбор альтернативы + уведомление
3. **Пустое выделение** → уведомление "Сначала выделите текст"
4. **Слишком длинный текст** → предложение разбить или сократить
5. **Rate limit превышен** → показать время ожидания
6. **API недоступен** → повтор с exponential backoff + уведомление
7. **Буфер обмена заблокирован** → повтор через 100ms до 5 раз
8. **Неверный API ключ** → показать окно настроек

### UI уведомления об ошибках

```rust
pub enum NotificationType {
    Success(String),
    Warning { message: String, action: Option<Action> },
    Error { message: String, action: Option<Action> },
}

pub enum Action {
    OpenSettings,
    SuggestHotkey(KeyCombo),
    RetryTranslation,
    SplitText,
}

impl TrayApp {
    pub fn show_notification(&self, notification: NotificationType) {
        match notification {
            NotificationType::Error { message, action } => {
                // Показываем ошибку с кнопкой действия
                if let Some(action) = action {
                    self.show_actionable_notification(&message, action);
                } else {
                    self.show_error(&message);
                }
            }
            // ...
        }
    }
}
```

## Метрики качества (обновленные)

- Время отклика < 100ms (от нажатия до начала обработки)
- Успешность переводов > 99%
- Обработка edge cases > 99.9%
- Детекция конфликтов горячих клавиш = 100%
- Размер приложения < 10MB
- Потребление RAM < 50MB
- CPU в idle < 0.1%

## Тесты для Edge Cases _(новые)_

### Unit тесты

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_selection() {
        let mut mgr = ClipboardManager::new();
        let result = mgr.get_selection_sync("");
        assert!(matches!(result, Err(SelectionError::EmptySelection)));
    }

    #[test]
    fn test_whitespace_only() {
        let mut mgr = ClipboardManager::new();
        let result = mgr.get_selection_sync("   \n\t  ");
        assert!(matches!(result, Err(SelectionError::OnlyWhitespace)));
    }

    #[test]
    fn test_text_too_long() {
        let validator = TextValidator::new(1000, 250);
        let long_text = "a".repeat(2000);
        let result = validator.validate(&long_text);
        assert!(matches!(result, Ok(ValidationResult::TooLong { .. })));
    }

    #[test]
    fn test_hotkey_conflict() {
        let validator = HotkeyValidator::new();
        let system_combo = KeyCombo::new(&[Key::Alt, Key::Tab]);
        let result = validator.validate(&system_combo);
        assert_eq!(result, ValidationResult::SystemConflict);
    }

    #[test]
    fn test_rate_limit() {
        let mut limiter = RateLimiter::new(10, 100);

        // Заполняем лимит
        for _ in 0..10 {
            assert!(limiter.check_and_update().is_ok());
        }

        // Следующий должен fail
        assert!(matches!(
            limiter.check_and_update(),
            Err(RateLimitError::MinuteLimit { .. })
        ));
    }
}
```

!!! не забывай отмечать выполненные пункты галочками в CLAUDE.md и иди точно по плану.
Все размышления и разговоры в терминале веди на русском языке.
