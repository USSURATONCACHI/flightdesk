# Flightdeck — Русский
language-name = Русский

## Common words
common-remove = Удалить
common-reset = сброс
common-on = Вкл
common-off = Выкл
common-enabled = Включён
common-none = нет
common-version = версия
common-cancel = Отмена
common-close = Закрыть
common-copy = Копировать
common-copied = скопировано
common-details = Подробности
common-yes = да
common-no = нет
common-ok = ОК
common-loading = загрузка…
common-error = Ошибка
common-retry = Повторить
common-search = Поиск
common-open = Открыть
common-open-new-tab = Открыть в новой вкладке
common-refresh = Обновить
common-all = все
common-unknown = неизвестно

## Themes & density
theme-system = Системная
theme-light = Светлая
theme-dark = Тёмная
density-compact = Компактно
density-normal = Обычно
density-comfortable = Просторно

## Connection fields
connect-name = Название
connect-rest-url = REST URL
connect-user = Пользователь
connect-password = Пароль
connect-remember = запомнить
connect-cockpit-url = URL Cockpit
connect-cockpit-url-hint = Используется командой «Скопировать ссылку Cockpit». Оставьте пустым, чтобы вывести из REST URL.
connect-allow-self-signed = Разрешить самоподписанный TLS
connect-connect = Подключиться
connect-connected = подключено
connect-add = + Добавить подключение
connect-default-name = движок { $n }

## Settings place
settings-title = Настройки
settings-saved-automatically = сохраняются автоматически
settings-connections = Подключения
settings-auto-connect = Подключаться к последнему движку при запуске
settings-appearance = Внешний вид
settings-theme = Тема
settings-language = Язык
settings-language-system = Системный
settings-language-none = языковые пакеты не найдены; показаны идентификаторы сообщений
settings-language-warnings = { $n ->
    [one] { $n } проблема в языковом пакете (наведите для подробностей)
    [few] { $n } проблемы в языковом пакете (наведите для подробностей)
   *[many] { $n } проблем в языковом пакете (наведите для подробностей)
}
settings-density = Плотность
settings-diagram-input = Управление диаграммой
settings-mouse-wheel = Колесо мыши
settings-wheel-zooms = масштабирует (Ctrl+колесо прокручивает)
settings-wheel-pans = прокручивает (Ctrl+колесо масштабирует)
settings-inertia = Инерция после броска диаграммы
settings-zoom-speed = Скорость масштабирования
settings-reset-to = вернуть { $value }×
settings-zoom-global = Общая
settings-zoom-global-tip = Масштабирует любой жест: колесо, щипок, диск масштаба и клавиатуру
settings-zoom-mouse = Мышь
settings-zoom-mouse-tip = Дополнительный множитель для жестов указателя (колесо, щипок, диск) поверх общего
settings-zoom-keyboard = Клавиатура
settings-zoom-keyboard-tip = Дополнительный множитель для клавиш + / − поверх общего
settings-zoom-alt = С зажатым Alt
settings-zoom-alt-tip = Скорость прокрутки и масштабирования при зажатом Alt поверх всего остального. Меньше 1 — замедление для точной работы
settings-zoom-effective = итого: мышь { $mouse }×, клавиатура { $keyboard }×, с Alt ещё ×{ $alt }
settings-unlimited-drag = Бесконечное перетаскивание
settings-unlimited-drag-on-tip = Переносить указатель к противоположному краю, когда перетаскивание доходит до границы окна
settings-unlimited-drag-off-tip = Перетаскивание останавливается у края
settings-find = Поиск по диаграмме (Ctrl+F)
settings-find-instances = Также искать запущенные экземпляры и подсвечивать активности, где они ждут
settings-find-business-keys = по бизнес-ключу (содержит)
settings-find-ids = по идентификатору экземпляра (содержит)
settings-refresh = Обновление
settings-refresh-instances = Экраны экземпляров (секунды)
settings-refresh-lists = Списки (секунды)
settings-page-size = Размер страницы
settings-pause-unfocused = Приостанавливать автообновление, пока окно не в фокусе
settings-confirm-destructive = Подтверждать необратимые действия
settings-mcp = MCP-сервер (ИИ-агенты)
settings-mcp-description = Открывает Flightdeck ИИ-агентам по Model Context Protocol (Streamable HTTP на localhost). Инструменты охватывают движок (определения, экземпляры, переменные, инциденты, задания, задачи, модификацию, пакеты) и сам интерфейс (навигация, выбор, масштаб, панель, поиск, снимки экрана, клавиши).
settings-mcp-port = Порт
settings-mcp-token = Bearer-токен (необязательно)
settings-mcp-listening = слушает { $url }
settings-mcp-copy-url = Скопировать URL
settings-mcp-copy-config = Скопировать конфигурацию Claude Code
settings-mcp-stats = запросов: { $requests } · вызовов инструментов: { $calls } · ошибок: { $errors }
settings-mcp-last = последний: { $tool }
settings-saved-searches = Сохранённые запросы
settings-saved-searches-empty = Сохраните запрос на экране поиска (☆).
settings-about = О программе
settings-about-text = Flightdeck — быстрый кокпит для Camunda 7 с управлением с клавиатуры.
settings-file = файл настроек
settings-not-persisted = настройки в этом сеансе не сохраняются

## Connection screen
connect-title = Подключения
connect-list-hint = Клик · Ctrl добавляет · Shift выделяет диапазон · Alt снимает · перетащите в корзину
connect-unnamed = (без названия)
connect-add-tip = Добавить подключение
connect-new-name = Подключение { $n }
connect-empty = Сохранённых подключений пока нет.
connect-bin-release = Отпустите, чтобы удалить
connect-bin-hint = Перетащите сюда подключения для удаления
connect-delete-one = Удалить подключение «{ $name }»?
connect-delete-many = Удалить подключений: { $n }?
    • { $list }
connect-delete-title = Удаление подключений
connect-deleted = { $n ->
    [one] Удалено { $n } подключение
    [few] Удалено { $n } подключения
   *[many] Удалено { $n } подключений
}
connect-subtitle = Подключение к движку Camunda 7
connect-select-hint = Выберите подключение слева или добавьте новое кнопкой «+».
connect-user-hint = необязательно (basic auth)
connect-show-password = Показать пароль
connect-remember-password = Запомнить пароль
connect-remember-tip = Хранится в файле настроек (читать может только владелец)
connect-connecting = Подключение…
connect-url-note = URL нормализуется: подойдут `host:8080`, `…/camunda/app/cockpit/…` и `…/engine-rest`.
connect-open-bpmn = Открыть файл .bpmn
connect-open-bpmn-tip = Автономный просмотр: движок не нужен (путь из $FLIGHTDECK_BPMN)
common-delete = Удалить
common-suspend = Приостановить
common-activate = Активировать

## Toasts and errors
toast-connected = Подключено к Camunda { $version }
error-auth = Ошибка аутентификации: проверьте пользователя и пароль.
error-not-camunda = 404 на /version: по этому URL нет REST API Camunda.
toast-mcp-listening = MCP-сервер слушает { $url }
toast-mcp-failed = MCP-сервер не смог запуститься на порту { $port }
toast-mcp-stopped = MCP-сервер остановлен
toast-no-definition-key = Нет определения с ключом { $key }
toast-copied = Скопировано: { $what }
toast-job-retries = Число попыток задания: { $n }
toast-job-retries-failed = Не удалось задать число попыток задания
toast-external-retries = Число попыток внешней задачи: { $n }
toast-external-retries-failed = Не удалось задать число попыток внешней задачи
definition-suspend-title = Приостановить определение
definition-suspend-body = Приостановить определение процесса { $id } вместе со всеми запущенными экземплярами?
definition-activate-title = Активировать определение
definition-activate-body = Активировать определение процесса { $id } вместе со всеми запущенными экземплярами?
toast-definition-suspended = Определение приостановлено
toast-definition-activated = Определение активировано
toast-suspension-failed = Не удалось изменить состояние приостановки
perms-warning = { $n ->
    [one] Файл настроек доступен другим пользователям ({ $n } путь). Сохранённые пароли должны быть приватными.
    [few] Файлы настроек доступны другим пользователям ({ $n } пути). Сохранённые пароли должны быть приватными.
   *[many] Файлы настроек доступны другим пользователям ({ $n } путей). Сохранённые пароли должны быть приватными.
}
perms-fix = Исправить права
perms-fixed = Каталог настроек теперь приватный (700 / 600)
perms-fix-failed = Часть прав изменить не удалось

## Top bar, status bar, tabs
status-connecting = подключение…
status-offline = не подключено
topbar-refresh-everything = Обновить всё
topbar-disconnect = Отключиться
tab-close-tip = Закрыть (Ctrl+W)
tab-new-tip = Новая вкладка (Ctrl+T)
topbar-shortcuts-tip = Горячие клавиши (?)
topbar-mcp-off = MCP выкл
topbar-mcp-tip = Встроенный MCP-сервер для ИИ-агентов. Нажмите, чтобы переключить.
topbar-settings-tip = Настройки (Ctrl+,)
topbar-theme-tip = Тема: системная / светлая / тёмная (Ctrl+Shift+T переключает)
topbar-search = Поиск или переход…
nav-forward-tip = Вперёд (Alt+→)
nav-back-tip = Назад (Alt+←)
refresh-resume-tip = Возобновить автообновление (Ctrl+Shift+P)
refresh-pause-tip = Приостановить автообновление (Ctrl+Shift+P)
refresh-now-tip = Обновить сейчас (R / F5)
refresh-updating = обновление…
refresh-updated = обновлено { $ago }
status-requests = { $n ->
    [one] { $n } запрос
    [few] { $n } запроса
   *[many] { $n } запросов
}
status-paused = автообновление приостановлено
status-paused-unfocused = обновление приостановлено: окно не в фокусе

## Diagram context menu
ctx-inspect = Просмотреть
ctx-copy-id = Копировать id
ctx-copy-name = Копировать название
ctx-instances-waiting = Экземпляры, ожидающие здесь
ctx-instances-executed = Экземпляры, выполнявшие это…
ctx-open-called = Открыть вызываемый процесс { $key }
ctx-go-source = К источнику: { $name }
ctx-go-target = К цели: { $name }
ctx-diagram = Диаграмма
ctx-fit-diagram = Вписать диаграмму

## Commands (palette, help)
cat-diagram = Диаграмма
cat-general = Общее
cat-navigate = Навигация
cat-tabs = Вкладки
cat-view = Вид
cmd-back = Назад
cmd-close-tab = Закрыть вкладку
cmd-copy-cockpit-link = Скопировать ссылку для Camunda Cockpit
cmd-copy-place-id = Скопировать текущий id (определение / экземпляр)
cmd-disconnect = Отключиться от движка
cmd-find-in-diagram = Найти элемент на диаграмме
cmd-fit-diagram = Вписать диаграмму
cmd-focus-filter = Перейти к фильтру / полю поиска
cmd-forward = Вперёд
cmd-go-batches = К пакетам
cmd-go-definitions = К определениям процессов
cmd-go-incidents = К инцидентам
cmd-help = Горячие клавиши
cmd-new-tab = Новая вкладка
cmd-next-tab = Следующая вкладка
cmd-open-settings = Открыть настройки
cmd-palette = Палитра команд
cmd-prev-tab = Предыдущая вкладка
cmd-refresh = Обновить сейчас
cmd-search-instances = Поиск экземпляров процессов
cmd-toggle-dock = Показать/скрыть боковую панель
cmd-toggle-pause = Приостановить / возобновить автообновление
cmd-toggle-theme = Сменить тему: системная → светлая → тёмная
cmd-zoom-in = Приблизить
cmd-zoom-out = Отдалить

## Palette
palette-open-instance = Открыть экземпляр { $id }
palette-kind-instance-id = id экземпляра процесса
palette-open-as-definition = Открыть как id определения
palette-open-definition = Открыть определение { $id }
palette-kind-definition-id = id определения процесса
palette-search-business-key = Искать экземпляры по бизнес-ключу «{ $q }»
palette-search-instances = Искать экземпляры: { $q }
common-no-matches = Ничего не найдено
toast-refresh-paused = Автообновление приостановлено
toast-refresh-resumed = Автообновление возобновлено

## Help overlay
help-title = Клавиатура и мышь
help-diagram = Диаграмма
help-everywhere = Везде
help-close-hint = ? или Esc — закрыть
help-g-dock-tab = вкладка панели
help-g-tables = таблицы: перемещение · открыть · открыть в новой вкладке
help-g-middle-click = открыть в новой вкладке
help-g-slash = перейти к полю фильтра
help-g-alt = просмотр элемента под курсором
help-d-wheel = масштаб у курсора
help-d-alt = быстрее прокрутка и масштаб (×2 по умолчанию, см. Настройки)
help-d-shift-wheel = горизонтальная прокрутка
help-d-drag = прокрутка (бросок — инерция)
help-d-arrows = прокрутка (удерживайте для ускорения, Shift быстрее)
help-d-plus-minus = приблизить / отдалить (Alt: на множитель Alt)
help-d-fit = вписать всё · 100 % · вписать выделение · вписать токены
help-d-double-click = приблизить к элементу (пусто: вписать всё)
help-d-tab = следующий / предыдущий элемент по потоку
help-d-alt-arrows = по исходящему / входящему потоку (1-9 выбирает)
help-d-enter = просмотр · переход в вызываемый процесс
help-d-find = найти элемент или экземпляр (Enter/Shift+Enter листают)
help-d-copy = копировать id элемента
help-d-toggles = миникарта · сетка · id элементов
help-d-right-click = контекстное меню
help-d-esc = закрыть поиск → снять выделение

## Diagram: find bar, zoom, tooltips
pick-banner = { $label } — щёлкните целевой элемент, Esc отменяет
find-hint = Элемент, бизнес-ключ или id экземпляра…
find-no-matches = ничего не найдено
find-prev-tip = Предыдущее совпадение (Shift+Enter)
find-next-tip = Следующее совпадение (Enter)
find-close-tip = Закрыть (Esc)
find-case-tip = С учётом регистра
find-flows-tip = Включая потоки последовательности
find-cat-task = задача
find-cat-gateway = шлюз
find-cat-event = событие
find-cat-call = вызов
find-cat-subprocess = подпр.
find-only-task = Только задачи
find-only-gateway = Только шлюзы
find-only-event = Только события
find-only-call = Только вызовы процессов
find-only-subprocess = Только подпроцессы
find-count-elements = { $n ->
    [one] { $n } элемент
    [few] { $n } элемента
   *[many] { $n } элементов
}
find-searching-instances = поиск экземпляров…
find-count-instances = { $n ->
    [one] { $n } экземпляр
    [few] { $n } экземпляра
   *[many] { $n } экземпляров
} в { $a ->
    [one] { $a } активности
   *[other] { $a } активностях
}
find-legend-instance-short = совпадение экземпляра
find-go-to = Перейти к { $name }
zoom-tip = Масштаб. Клик — пресеты. Колесо масштабирует, Shift+колесо прокручивает, 0 вписывает, 1 = 100 %
zoom-fit-diagram = Вписать диаграмму
zoom-fit-selection = Вписать выделение
zoom-minimap = Миникарта
zoom-grid = Сетка
zoom-ids = Id элементов
zoom-dial-tip = Тяните влево или вправо, чтобы масштабировать без ограничений
tip-running = { $n } выполняется
tip-incidents = { $n ->
    [one] { $n } инцидент
    [few] { $n } инцидента
   *[many] { $n } инцидентов
}
tip-executed = выполнялось ×{ $n }
tip-timer = таймер { $t }
tip-default-flow = поток по умолчанию
find-explain-element = Совпадение элемента: { $field } «{ $snippet }»
find-explain-instances = { $n ->
    [one] { $n } запущенный экземпляр здесь совпадает:
    [few] { $n } запущенных экземпляра здесь совпадают:
   *[many] { $n } запущенных экземпляров здесь совпадают:
}
find-explain-business-key = бизнес-ключ { $key }
find-explain-id = id { $id }
find-explain-join = Токен этого экземпляра ждёт в этом соединении прихода остальных ветвей.
find-explain-event-gateway = Экземпляр ждёт здесь одно из следующих событий.
find-explain-more = … и ещё { $n }
find-explain-striped = штриховка: совпадают и элемент, и экземпляр
find-explain-current = Текущее совпадение { $i } из { $n } (Enter / Shift+Enter листают)
find-legend-element = совпадает текст элемента ({ $n })
find-legend-instance = здесь совпадает запущенный экземпляр ({ $n })
find-legend-both = и то и другое ({ $n })
find-legend-current = текущее совпадение · Enter листает

## Places, lists and hints
place-definitions = Определения
place-incidents = Инциденты
place-batches = Пакеты
definitions-title = Определения процессов
definitions-load-failed = Не удалось загрузить определения: { $error }
definitions-empty = Определения процессов не развёрнуты.
definitions-copy-key = Копировать ключ
definitions-copy-id = Копировать id определения
common-all-versions = Все версии
common-nothing-matches-filter = Ничего не подходит под фильтр.
common-search-instances = Искать экземпляры
common-copy-business-key = Копировать бизнес-ключ
common-open-definition = Открыть определение
common-select-on-diagram = Выбрать на диаграмме
common-click-to-copy = Нажмите, чтобы скопировать
common-find = Найти
common-save = Сохранить
common-set = Задать
common-edit = Изменить
common-clear = Очистить
common-apply = Применить
common-restart = Перезапустить
common-resume = Возобновить
common-prev = назад
common-next = далее
common-now = сейчас
common-undo = Отменить
state-suspended = приостановлено
state-running-lower = выполняется
n-running = { $n } выполняется
n-incidents = { $n ->
    [one] { $n } инцидент
    [few] { $n } инцидента
   *[many] { $n } инцидентов
}
n-tokens = { $n ->
    [one] { $n } токен
    [few] { $n } токена
   *[many] { $n } токенов
}
n-open = открыто: { $n }
n-days = { $n ->
    [one] { $n } день
    [few] { $n } дня
   *[many] { $n } дней
}
hint-list = ↑↓ перемещение · Enter открыть · Shift+Enter в новой вкладке · / фильтр
hint-definition = Колесо — масштаб · перетаскивание — прокрутка · Tab — следующий элемент · Enter — просмотр / переход в вызываемый процесс · клик по значку фильтрует экземпляры · Ctrl+F — поиск · ? — справка
hint-instance = Shift+F — вписать токены · Enter — просмотр / переход в вызванный экземпляр · правая кнопка: модификация токенов · Ctrl+1…9 — вкладки панели · ? — справка
hint-pick = Выберите целевой элемент на диаграмме · Esc отменяет
hint-incidents = ↑↓ перемещение · Enter открыть экземпляр · Shift+Enter в новой вкладке · / фильтр
hint-search = ↑↓ перемещение · Enter открыть · Shift+Enter в новой вкладке · Пробел выделить · Ctrl+A все · двойной клик открыть · / запрос
hint-file = Перетаскивание/колесо: прокрутка и масштаб · 0 вписать · F вписать выделение · Tab следующий элемент · Ctrl+F поиск · ? справка
definition-show-instances = Показать экземпляры
definition-find-element = Найти элемент
definition-search-instances-tip = Открыть поиск экземпляров для этого определения
diagram-load-failed = Не удалось загрузить диаграмму: { $error }
diagram-loading = Загрузка диаграммы…
inspector-empty-definition = Выберите элемент для просмотра.
    Tab перебирает элементы · Ctrl+F ищет · удерживайте Alt для просмотра при наведении.
inspector-empty-instance = Выберите элемент для просмотра.
    Tab перебирает элементы · удерживайте Alt для просмотра при наведении.
inspector-empty-file = Выберите элемент для просмотра.
    Tab перебирает элементы, Ctrl+F их ищет.
toast-element-id-copied = Id элемента скопирован
instances-at = в
instances-clear-activity-filter = Сбросить фильтр по активности
instances-empty = Нет подходящих запущенных экземпляров.
definition-no-incidents = Открытых инцидентов в этой версии нет.
instance-short = экземпляр { $id }
incident-retry-job-tip = Установить число попыток задания = 1
incident-retry-external-tip = Установить число попыток внешней задачи = 1
incident-retry-tip = Установить число попыток = 1
incidents-group-by-message = Группировать по сообщению
incidents-filter-hint = Фильтр по сообщению, типу, определению, активности
incidents-empty = Открытых инцидентов нет. 🎉
incidents-none = Инцидентов нет.
batches-running = Выполняются
batches-finished = Завершённые (история)
batches-empty-running = Нет выполняющихся пакетов.
batches-empty-finished = Нет завершённых пакетов.
batch-delete-title = Удалить пакет
batch-delete-body = Удалить этот пакет и его оставшиеся задания? Уже выполненные задания не отменяются.
toast-batch-deleted = Пакет удалён
toast-batch-delete-failed = Не удалось удалить пакет
toast-batch-updated = Пакет обновлён
toast-batch-update-failed = Не удалось обновить пакет
started-when = запущен { $when }
created-when = создана { $when }
created-by = создал { $user }
file-diagram-warnings = { $n ->
    [one] { $n } предупреждение диаграммы (см. вкладку Инспектор)
    [few] { $n } предупреждения диаграммы (см. вкладку Инспектор)
   *[many] { $n } предупреждений диаграммы (см. вкладку Инспектор)
}
file-open-failed = Не удалось открыть { $path }: { $error }
inspector-running-here = здесь выполняется: { $n }
inspector-running-here-tip = Показать экземпляры, ожидающие на этом элементе
inspector-executed-tip = Поиск по истории: выполненная активность
mi-sequential = последовательно
mi-parallel = параллельно
io-inputs = Входы
io-outputs = Выходы
inspector-copy-xml = Копировать XML
confirm-hint = Tab — к кнопке подтверждения, или нажмите y
toast-hide-details = Скрыть подробности

## Instance place
instance-tab-title = экз. { $id }
job-failed = сбой
instance-super-tip = Родительский экземпляр { $id }
instance-copy-bk-tip = Нажмите, чтобы скопировать бизнес-ключ
toast-instance-id-copied = Id экземпляра скопирован
instance-tokens-tip = Shift+F вписывает камеру по токенам
instance-not-found = Экземпляр не найден (удалён или история очищена)
find-element-tip = Найти элемент (Ctrl+F)
instance-loading = Загрузка экземпляра…
trace-filter-hint = фильтр строк
extra-task = задача: { $who }
assigned-to = назначена { $user }
task-unassigned = не назначена
due-when = срок { $when }
extra-job = задание: срок { $due } · попыток { $retries }
extra-external = внешняя: тема { $topic } · { $lock }
locked-by = захвачена { $worker }
ext-unlocked = свободна
ctx-modify = Модификация
mod-start-before = Запустить токен перед
mod-start-after = Запустить токен после
mod-cancel-here = Отменить токены здесь
mod-move-from = Переместить токен отсюда в…
token = токен
token-async = асинхронное продолжение
incident-of-type = инцидент ({ $type })
instance-restart = Перезапустить…
instance-restart-tip = Запустить новый экземпляр по истории этого (вкладка Модификация)
instance-actions = Действия
instance-activate = Активировать экземпляр
instance-suspend = Приостановить экземпляр
toast-instance-activated = Экземпляр активирован
toast-instance-suspended = Экземпляр приостановлен
instance-retry-failed-jobs = { $n ->
    [one] Повторить { $n } сбойное задание
    [few] Повторить { $n } сбойных задания
   *[many] Повторить { $n } сбойных заданий
}
instance-retry-failed-external = { $n ->
    [one] Повторить { $n } сбойную внешнюю задачу
    [few] Повторить { $n } сбойные внешние задачи
   *[many] Повторить { $n } сбойных внешних задач
}
instance-modify-tokens = Модифицировать токены…
instance-delete = Удалить экземпляр…
instance-delete-title = Удаление экземпляра процесса
instance-delete-body = Удалить экземпляр { $id } { $bk }?
    Все его токены, задачи и задания будут удалены. Это необратимо.
toast-instance-deleted = Экземпляр удалён
toast-instance-delete-failed = Не удалось удалить экземпляр
tokens-none = Активных токенов нет
tokens-in = в { $path }
tokens-finished = Завершён — токенов нет
scope-process = процесс
scope-task = задача { $id }
scope-execution = исполнение { $id }
vars-add = + Добавить
vars-copy-json = Копировать JSON
vars-copy-json-tip = Скопировать все переменные как объект JSON
toast-vars-copied = Переменные скопированы как JSON
vars-name-hint = имя
vars-date-format = формат: 2026-10-20T10:00:00.000+0300
vars-name-required = Нужно указать имя
toast-var-restored = Переменная { $name } восстановлена
toast-undo-failed = Отменить не удалось
toast-var-saved = Переменная { $name } сохранена
toast-var-save-failed = Не удалось сохранить переменную { $name }
vars-empty = Переменных нет.
vars-no-match = Ни одна переменная не подходит под фильтр.
vars-row-tip = Клик — развернуть · двойной клик — копировать имя
vars-deser-error = ошибка десериализации: { $error }
vars-delete-title = Удаление переменной
vars-delete-body = Удалить переменную «{ $name }» из этого экземпляра?
toast-var-deleted = Переменная { $name } удалена
toast-var-delete-failed = Не удалось удалить { $name }
vars-err-integer = не целое число: { $error }
vars-err-boolean = укажите true или false
vars-err-date = нераспознанная дата
vars-err-json = некорректный JSON: { $error }
tasks-finished = Экземпляр завершён — открытых задач нет.
tasks-empty = Открытых пользовательских задач нет.
task-claim = Взять
task-claim-tip = Взять как { $user }
task-unclaim = Отпустить
toast-task-unassigned = Назначение задачи снято
toast-assign-failed = Не удалось назначить исполнителя
toast-task-claimed = Задача взята
toast-claim-failed = Не удалось взять задачу
toast-task-unclaimed = Задача отпущена
toast-unclaim-failed = Не удалось отпустить задачу
task-due-plus-day = Срок +1 день
task-clear-due = Снять срок
toast-due-tomorrow = Срок установлен на завтра
toast-due-failed = Не удалось установить срок
toast-due-cleared = Срок снят
toast-due-clear-failed = Не удалось снять срок
toast-due-now = Срок установлен на сейчас
toast-due-plus-hour = Срок сдвинут на час
task-complete = Завершить…
task-complete-tip = Завершить задачу без переменных
task-complete-title = Завершение задачи
task-complete-body = Завершить задачу «{ $name }» без переменных? Процесс продолжится.
task-complete-button = Завершить
toast-task-completed = Задача завершена
toast-task-complete-failed = Не удалось завершить задачу
inc-resolved = решён
inc-deleted = удалён
inc-closed = закрыт
inc-retry-1 = Повторить (попыток=1)
inc-retries-3 = Попыток=3
inc-stack-trace = Трассировка стека
inc-error-details = Подробности ошибки
trace-job-title = Трассировка стека задания { $id }
trace-external-title = Подробности ошибки внешней задачи { $id }
inc-annotate = Аннотировать…
inc-resolve = Решить
inc-resolve-tip = Удалить инцидент (причину не устраняет)
toast-incident-resolved = Инцидент решён
toast-incident-resolve-failed = Не удалось решить инцидент
inc-annotation-title = Аннотация инцидента
toast-annotation-saved = Аннотация сохранена
toast-annotation-failed = Не удалось сохранить аннотацию
trace-load-failed = Не удалось загрузить: { $error }
jobs-finished = Экземпляр завершён — заданий нет.
jobs-empty = Заданий нет (таймеров и асинхронных продолжений).
job-execute = Выполнить сейчас
job-execute-tip = Выполнить задание синхронно
toast-job-executed = Задание выполнено
toast-job-execute-failed = Выполнение задания не удалось
job-retries-1 = Попыток → 1
job-retries-3 = Попыток → 3
job-due-now = Срок сейчас
job-due-plus-hour = Срок +1 ч
toast-job-suspension = Приостановка задания изменена
toast-job-change-failed = Не удалось изменить задание
ext-finished = Экземпляр завершён — внешних задач нет.
ext-empty = Внешних задач нет.
ext-not-locked = — (не захвачена)
ext-never-failed = — (сбоев не было)
ext-unlock = Освободить
toast-ext-unlocked = Внешняя задача освобождена
toast-ext-unlock-failed = Не удалось освободить
ext-priority-plus = Приоритет +10
toast-priority-raised = Приоритет повышен
toast-priority-failed = Не удалось задать приоритет
called-empty = Вызванных экземпляров процессов нет.
timeline-empty = Истории активностей пока нет.
timeline-start = начало { $when }
timeline-end = конец  { $when }  ({ $duration })
timeline-active = ещё активна
timeline-cancelled = отменена
timeline-assignee = исполнитель { $user }
timeline-called = вызванный экземпляр { $id }
mod-added-start-before = Добавлено: запуск перед { $name }
mod-added-start-after = Добавлено: запуск после { $name }
mod-added-cancel = Добавлено: отмена токенов в { $name }
mod-move-label = Переместить токен из { $name }
mod-added-move = Добавлено: переместить токен { $from } → { $to }
mod-restart-title = Перезапуск завершённого экземпляра
mod-restart-help = Укажите, где должен начаться новый экземпляр: правой кнопкой по элементу диаграммы → «Запустить токен перед/после». Переменные на момент завершения копируются, если «исходные переменные» выключены.
mod-title = Модификация токенов
mod-help = Правой кнопкой по элементу диаграммы: запустить токен перед/после него, отменить токены там или переместить токен. Ничего не отправляется, пока вы не примените.
mod-empty = Инструкций пока нет.
mod-skip-listeners = Пропустить пользовательские слушатели
mod-skip-io = Пропустить I/O-маппинги
mod-annotation-hint = аннотация (необязательно, попадёт в журнал операций)
mod-restart-button = Перезапустить экземпляр…
mod-apply-button = Применить модификацию…
mod-restart-body = Запустить новый экземпляр этого процесса с:
    { $list }

    Переменные копируются из завершённого экземпляра.
mod-apply-body = Применить к экземпляру { $id }:
    { $list }
mod-restart-confirm-title = Перезапуск экземпляра
mod-confirm-title = Модификация токенов
toast-restarted = Экземпляр перезапущен (см. список экземпляров определения)
toast-restart-failed = Перезапуск не удался
toast-modified = Модификация применена
toast-modify-failed = Модификация не удалась

## Search place
search-syntax-title = Синтаксис запроса
search-syntax-operators = Операторы: поле:значение (=), != > >= < <= и ~ (like, * подстановка). Значения с пробелами — в кавычках. Одиночное слово ищет по бизнес-ключам; одиночный uuid — id экземпляра.
search-examples = Примеры
search-preview-empty = Наведите курсор на результат, чтобы увидеть его переменные.
search-variables = Переменные
search-add-column = + столбец
search-add-column-tip = Добавить как столбец результатов
search-query-hint = def:ключ  bk:ABC*  var.amount>1000  act:Task_x  state:finished  …   (Enter — искать, Tab — принять подсказку)
search-suggest-hint = Tab принимает · Enter ищет
search-mode-runtime = runtime
search-mode-history = история
search-mode-auto = авто
search-mode-tip = Какой API отвечает: runtime — только запущенные экземпляры, история — запущенные и завершённые. Нажмите, чтобы зафиксировать.
search-save-query = Сохранить текущий запрос…
search-no-saved = Сохранённых запросов нет
search-results = { $n ->
    [one] { $n } результат
    [few] { $n } результата
   *[many] { $n } результатов
}
search-var-column-hint = + столбец переменной
search-remove-column = Убрать столбец
search-empty-hint = Введите запрос и нажмите Enter. Список полей — на вкладке «Синтаксис» в панели.
search-no-match = Подходящих экземпляров нет.
search-var-col = var { $name }
running-for = выполняется { $duration }
search-selected = выделено: { $n }
search-clear-selection = Снять выделение
batch-suspend-title = Приостановить экземпляры
batch-suspend-body = Приостановить пакетом экземпляров: { $n }?
batch-activate-title = Активировать экземпляры
batch-activate-body = Активировать пакетом экземпляров: { $n }?
batch-retry-jobs = Повторить сбойные задания
batch-retry-title = Повтор заданий
batch-retry-body = Установить пакетом число попыток = 1 для экземпляров: { $n }?
batch-set-variable = Задать переменную…
batch-set-variable-hint = Введите имя=значение в поле переменной и нажмите в нём Enter при выделенных строках (только строки).
batch-delete = Удалить…
batch-delete-instances-title = Удаление экземпляров
batch-delete-instances-body = Удалить пакетом экземпляров процессов: { $n }? Это необратимо.
toast-batch-started = Пакет { $id } запущен (заданий: { $jobs }). См. Пакеты (Ctrl+Shift+B).
toast-batch-start-failed = Не удалось запустить пакет

## Dock tabs
dock-inspector = Инспектор
dock-instances = Экземпляры
dock-incidents = Инциденты
dock-versions = Версии
dock-overview = Обзор
dock-inspect = Просмотр
dock-variables = Переменные
dock-tasks = Задачи
dock-jobs = Задания
dock-external = Внешние
dock-called = Вызванные
dock-timeline = Хронология
dock-modify = Модификация
dock-preview = Предпросмотр
dock-syntax = Синтаксис
dock-matches = Совпадения

## Table columns
col-name = Название
col-key = Ключ
col-version = Версия
col-running = Выполняется
col-incidents = Инциденты
col-state = Состояние
col-business-key = Бизнес-ключ
col-started = Запущен
col-id = Id
col-deployed = Развёрнуто
col-type = Тип
col-jobs = Задания
col-ended = Завершён
col-time = Время
col-definition = Определение
col-activity = Активность
col-message = Сообщение
col-instance = Экземпляр
col-ended-duration = Завершён / длительность
col-business-key-id = Бизнес-ключ / id
col-via = Через
col-flow = поток
col-target = цель
col-condition = условие

## Inspector sections
section-identity = Идентификация
section-runtime = Выполнение
section-in-this-instance = В этом экземпляре
section-execution = Исполнение
section-implementation = Реализация
section-user-task = Пользовательская задача
section-form = Форма
section-event-definition = Определение события
section-outgoing-flows = Исходящие потоки
section-connections = Связи
section-multi-instance = Мультиэкземпляр
section-input-output-mapping = Маппинг входов / выходов
section-listeners = Слушатели
section-extension-properties = Свойства расширения
section-other-attributes = Прочие атрибуты
section-xml = XML
section-definition = Определение
section-instance = Экземпляр
section-called-by = Вызван из
section-condition = Условие
section-now = Сейчас

## Key/value row labels
kv-id = id
kv-name = название
kv-key = ключ
kv-type = тип
kv-version = версия
kv-documentation = документация
kv-lane = дорожка
kv-pool = пул
kv-parent = родитель
kv-attached-to = прикреплено к
kv-interrupting = прерывающее
kv-boundary-event = граничное событие
kv-executed-history = выполнялось (история)
kv-retry-time-cycle = цикл повторов
kv-called-element = вызываемый элемент
kv-script-format = формат скрипта
kv-script = скрипт
kv-message = сообщение
kv-message-ref = ссылка на сообщение
kv-signal = сигнал
kv-signal-ref = ссылка на сигнал
kv-error = ошибка
kv-error-code = код ошибки
kv-code-variable = переменная кода
kv-message-variable = переменная сообщения
kv-escalation = эскалация
kv-escalation-code = код эскалации
kv-condition = условие
kv-language = язык
kv-variable-name = имя переменной
kv-variable-events = события переменной
kv-link = ссылка
kv-compensates = компенсирует
kv-wait-for-completion = ждать завершения
kv-default-flow = поток по умолчанию
kv-from = из
kv-to = в
kv-mode = режим
kv-collection = коллекция
kv-element-variable = переменная элемента
kv-cardinality = кардинальность
kv-completion-condition = условие завершения
kv-resource = ресурс
kv-deployment = развёртывание
kv-history-ttl = TTL истории
kv-description = описание
kv-business-key = бизнес-ключ
kv-definition = определение
kv-definition-id = id определения
kv-state = состояние
kv-started = запущен
kv-ended = завершён
kv-duration = длительность
kv-running-for = выполняется
kv-started-by = запустил
kv-start-activity = начальная активность
kv-delete-reason = причина удаления
kv-super-instance = родительский экземпляр
kv-root-instance = корневой экземпляр
kv-tenant = тенант
kv-history-removal = удаление истории
kv-task-id = id задачи
kv-due = срок
kv-follow-up = напоминание
kv-priority = приоритет
kv-assignee = исполнитель
kv-annotation = аннотация
kv-root-cause-incident = исходный инцидент
kv-in-instance = в экземпляре
kv-job-id = id задания
kv-retries = попыток
kv-configuration = конфигурация
kv-exception = исключение
kv-topic = тема
kv-worker = воркер
kv-lock-expires = блокировка до
kv-scope = область
kv-job-retries = попыток задания
kv-job-error = ошибка задания
kv-terminate = завершение
kv-cancel = отмена
kv-expression = выражение
kv-default = по умолчанию

## BPMN element kinds
bpmn-task = Задача
bpmn-user-task = Пользовательская задача
bpmn-service-task = Сервисная задача
bpmn-script-task = Скриптовая задача
bpmn-business-rule-task = Задача бизнес-правила
bpmn-send-task = Задача отправки
bpmn-receive-task = Задача получения
bpmn-manual-task = Ручная задача
bpmn-call-activity = Вызов процесса
bpmn-subprocess = Подпроцесс
bpmn-ad-hoc-subprocess = Ad-hoc подпроцесс
bpmn-transaction = Транзакция
bpmn-start-event = Начальное событие
bpmn-end-event = Конечное событие
bpmn-intermediate-catch-event = Промежуточное событие-приём
bpmn-intermediate-throw-event = Промежуточное событие-генерация
bpmn-boundary-event = Граничное событие
bpmn-exclusive-gateway = Эксклюзивный шлюз
bpmn-parallel-gateway = Параллельный шлюз
bpmn-inclusive-gateway = Инклюзивный шлюз
bpmn-event-based-gateway = Шлюз по событиям
bpmn-complex-gateway = Сложный шлюз
bpmn-pool = Пул
bpmn-lane = Дорожка
bpmn-data-object = Объект данных
bpmn-data-store = Хранилище данных
bpmn-data-input = Вход данных
bpmn-data-output = Выход данных
bpmn-text-annotation = Текстовая аннотация
bpmn-group = Группа
bpmn-event-timer = Таймер
bpmn-event-message = Сообщение
bpmn-event-signal = Сигнал
bpmn-event-error = Ошибка
bpmn-event-escalation = Эскалация
bpmn-event-conditional = Условное
bpmn-event-link = Ссылка
bpmn-event-compensation = Компенсация
bpmn-event-terminate = Завершение
bpmn-event-cancel = Отмена
bpmn-flow-sequence-flow = Поток последовательности
bpmn-flow-message-flow = Поток сообщений
bpmn-flow-association = Ассоциация
bpmn-flow-data-input-association = Ассоциация входа данных
bpmn-flow-data-output-association = Ассоциация выхода данных

## Stragglers
palette-hint = Команда, ключ или название определения, id экземпляра или бизнес-ключ…   (> команды, # бизнес-ключ)
connect-url-hint = http://host:8080/engine-rest  (подойдёт и URL Cockpit)
inc-resolve-title = Решить инцидент
inc-resolve-body = Отметить инцидент решённым? Исходное задание или задача не затрагиваются.
definitions-filter-hint = Фильтр по ключу или названию

## Time
time-ago = { $d } назад
time-in = через { $d }
time-just-now = только что
dur-ms = { $ms } мс
dur-s = { $s } с
dur-m-s = { $m } мин { $s } с
dur-h-m = { $h } ч { $m } мин
dur-d-h = { $d } д { $h } ч
bpmn-event-generic = событие

## Implementation / user-task attribute labels
kv-class = класс
kv-delegate-expression = delegate expression
kv-result-variable = переменная результата
kv-task-priority = приоритет задачи
kv-decision-ref = ссылка на решение
kv-decision-binding = привязка решения
kv-decision-version = версия решения
kv-decision-tenant = тенант решения
kv-map-decision-result = маппинг результата решения
kv-called-binding = привязка вызова
kv-called-version = версия вызова
kv-called-version-tag = тег версии вызова
kv-called-tenant = тенант вызова
kv-variable-mapping-class = класс маппинга переменных
kv-variable-mapping-delegate = делегат маппинга переменных
kv-candidate-users = кандидаты (пользователи)
kv-candidate-groups = кандидаты (группы)
kv-due-date = срок
kv-form-key = ключ формы
kv-form-ref = ссылка на форму
kv-form-binding = привязка формы
kv-form-version = версия формы
