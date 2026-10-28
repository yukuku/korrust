package newfamilies.arrays
fun typeNonnull(value: DoubleArray): DoubleArray = TypeUses.nonnull(value)
fun typeNullable(value: DoubleArray?): DoubleArray? = TypeUses.nullable(value)
fun typeText(value: String): String = TypeUses.text(value)
fun typeNullableText(value: String?): String? = TypeUses.nullableText(value)
fun typeCombined(value: DoubleArray): DoubleArray = TypeUses.combined(value)
fun typeCombinedNullable(value: DoubleArray?): DoubleArray? = TypeUses.combinedNullable(value)
