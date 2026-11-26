fun consume(): Boolean {
    val storage = Storage()
    val previous = Storage.exchange(storage)
    Storage.published = 99
    State.flag = true
    top = 12
    val counter = Counter(1)
    counter.value = 2
    return previous == null && Storage.exchange(null) === storage && Storage.published == 99L && State.flag && top == 12L && counter.value == 2
}
