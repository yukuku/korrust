package genericfields
fun <C> preserve(cell: Cell<C>): C = cell.value
fun <C> nullableView(cell: Cell<C>): Any? = cell.value
fun <C: Any> nonnull(cell: Cell<C>): C = cell.value
