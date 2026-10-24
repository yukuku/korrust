package twenty.consumer
import twenty.*
import twenty.dep.JavaApi
import twenty.dep.Trace
import yuku.alkitab.base.audio.*
import yuku.alkitab.songs.SongPlaybackState
private fun observe(id: String, action: () -> Unit) {
    action()
    println("{\"id\":\"$id\",\"outcome\":\"pass\"}")
}
fun main() {
    observe("top_level_constants") { check(ANSWER == 42 && BIG == Long.MAX_VALUE && ENABLED && LETTER == '\udfff' && FRACTION == 1.25f && java.lang.Float.floatToRawIntBits(NEGATIVE_ZERO) == Int.MIN_VALUE) }
    observe("java_static_calls") { check(staticCall(4, 5) == 9) }
    observe("java_fields") { val api = JavaApi(); check(fieldWrite(api, 9) == 9 && fieldRead(api) == 32) }
    observe("overload_resolution") { check(overloaded(1) == "int" && overloaded(1L) == "long" && overloaded(null) == "string") }
    observe("implicit_constructors") { check(Implicit().value() == 11) }
    observe("nonproperty_constructor_parameters") { val value = Captured(wide = 5L, seed = 2); check(value.first == 2 && value.second == 5L && value.third == 3) }
    observe("mutable_constructor_properties") { val value = MutableCtor(hidden = 2L, text = null, count = 1); value.count = 7; value.text = "x"; check(value.count == 7 && value.text == "x") }
    observe("initialized_instance_properties") { val value = Initialized(seed = 2); value.count = 5; check(value.count == 5 && value.next == 3 && value.label == "ready") }
    observe("source_construction_named_arguments") { check(explicitConstruct().third == 3) }
    observe("function_constructor_defaults") {
        Trace.events = ""; check(defaults(c = Trace.mark("C", 9), a = Trace.mark("A", 4)) == "4:3:9"); check(Trace.events == "CAb")
        Trace.events = ""; val value = DefaultCtor(c = Trace.mark("C", 9), a = Trace.mark("A", 4)); check(value.initialized == 9 && Trace.events == "CAbi")
        Trace.events = ""; check(DefaultMethods().compute(b = Trace.mark("B", 8)) == "3:8" && Trace.events == "Ba")
        check(DefaultCtor().c == 3 && plainDefault() == 7 && priorDefault(a = 8) == 9)
        check(mask32() == "1:16:32" && mask32(p32 = 90) == "1:16:90" && MaskCtor().total() == "1:16:32" && MaskCtor(p32 = 90).total() == "1:16:90")
    }
    observe("companion_initialization") { check(CompanionInit.FIRST == 4 && CompanionInit.INSTANCE.value == 6 && CompanionInit.INSTANCE === CompanionInit.INSTANCE) }
    observe("nullable_list_member_signatures") { val logs = listOf(AudioLogEntry(1L, "x")); val value = Signatures(null, logs); val result: List<AudioLogEntry> = value.entries(logs); val nullable: Int? = value.count(null); val surface: MemberSurface = MemberImplementation(logs); check(surface.nullable(null) == null && surface.logs() === logs); check(result === logs && nullable == null && value.nullableList == listOf<Int?>(null)) }
    observe("nullable_primitive_boxing") { val value: Int? = boxInt(10000); val wide: Long? = boxLong(Long.MIN_VALUE); val score: Float? = boxFloat(-0.0f); check(value == 10000 && wide == Long.MIN_VALUE && score == -0.0f && boxedBoolean(null) == null) }
    observe("primitive_safe_calls") { val value: Int? = safeLength(null); val flag: Boolean? = safeEmpty(""); check(nullableText(10000) == "10000" && nullableLongText(null) == null && chainedPrimitive("😀") == "2"); check(value == null && flag == true && primitiveElvis("😀") == 2) }
    observe("stable_local_smartcasts") { check(smart(null) == -1 && smartEarly("abc") == 3 && smartLocal("x") == 1) }
    observe("abrupt_elvis") { check(elvisReturn(null) == -1); try { elvisThrow(null); error("missing exception") } catch (expected: IllegalArgumentException) { check(expected.message == "missing") } }
    observe("typed_lists") { val entry = AudioLogEntry(1L, "x"); val value: AudioLogEntry = appList(listOf(entry)); check(value === entry && intList(listOf(10000)) == 10000 && longList(listOf(Long.MIN_VALUE)) == Long.MIN_VALUE) }
    observe("list_factories_mutation") { val values: MutableList<Int> = mutable(); check(mutate(values) == 17); values.add(22); check(values == listOf(8, 4, 9, 22) && empty().isEmpty()) }
    observe("rich_data_properties") {
        val logs = listOf(AudioLogEntry(2L, "x")); val value = RichRecord(optional = null, text = "a", score = Float.NaN, logs = logs)
        val (optional, text, score, entries) = value; check(optional == null && text == "a" && score.isNaN() && entries === logs)
        check(value.copy() == value && value.copy(optional = 2).optional == 2)
        val idle = PlaybackState.IDLE; check(idle.copy(bookId = 2).isActive && idle.logs.isEmpty() && idle.speed == 1.0f)
        check(SongPlaybackState.IDLE.copy(error = "x").error == "x")
    }
    observe("data_body_members") { check(RichRecord(null, null, 1.0f, emptyList()).summary() == "missing:0" && !PlaybackState.IDLE.isActive) }
}
