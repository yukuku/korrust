class Bad { val value: Int = 1
get() { field = 2; return field } }
