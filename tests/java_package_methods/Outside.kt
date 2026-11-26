package other
fun bad(store: app.Store): Int = store.read(1) + app.Store.next(1)
