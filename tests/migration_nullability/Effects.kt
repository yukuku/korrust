fun acceptNull(): String? = Migration.accept(null)
fun acceptNullable(value: String?): String? = Migration.accept(value)
fun nullableParameter(value: String?): String? = Migration.nullableParameter(value)
fun nonNullAsNonNull(missing: Boolean): String = Migration.nonNull(missing)
fun nonNullAsNullable(missing: Boolean): String? = Migration.nonNull(missing)
fun nullableAsNonNull(missing: Boolean): String = Migration.nullable(missing)
fun nullableAsNullable(missing: Boolean): String? = Migration.nullable(missing)
fun nullableLength(missing: Boolean): Int = Migration.nullable(missing).length
fun readNullableField(value: Migration): String? = value.nullableField
fun readNonNullField(value: Migration): String = value.nonNullField
fun writeFields(value: Migration, text: String?) {
    value.nullableField = text
    value.nonNullField = text
}
fun checkedStrictNonNull(): String = Migration.strictNonNull()
fun strictNullable(missing: Boolean): String? = Migration.strictNullable(missing)
fun generic(value: String?): String? = Migration.identity(value)
fun strictFieldNonNull(value: Migration): String = value.strictNonNullField
fun strictFieldNullable(value: Migration): String? = value.strictNullableField
fun staticField(): String? = Migration.staticField
fun writeStaticField(value: String?) { Migration.staticField = value }
