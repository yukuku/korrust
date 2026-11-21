package fieldmeta.dependency
@Target(AnnotationTarget.FIELD)
@Retention(AnnotationRetention.RUNTIME)
annotation class Label(val label: String, val count: Int, val names: Array<String>)
@Target(AnnotationTarget.FIELD)
@Retention(AnnotationRetention.BINARY)
annotation class BinaryLabel(val label: String)
@Target(AnnotationTarget.TYPE)
@Retention(AnnotationRetention.RUNTIME)
annotation class TypeLabel(val value: String)
enum class Choice {
    @Label("one", 3, ["x", "y"]) FIRST,
    @Label("two", 4, []) @BinaryLabel("hidden-label") SECOND;
    val typed: @TypeLabel("shape") String = ""
}
enum class Hidden {
    @Deprecated("hidden", level = DeprecationLevel.HIDDEN) ENTRY,
}
object Ordinary {
    @JvmField @Label("ordinary", 9, []) val text: String = "ordinary"
}

@RequiresOptIn
@Target(AnnotationTarget.FIELD)
@Retention(AnnotationRetention.BINARY)
annotation class NeedsConsent
enum class Restricted { @NeedsConsent ENTRY }
