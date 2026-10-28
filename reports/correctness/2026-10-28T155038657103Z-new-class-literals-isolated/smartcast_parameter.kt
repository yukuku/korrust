fun <T> boundary(value: T): kotlin.reflect.KClass<*> { if (value != null) return value::class; return String::class }
