@file:OptIn(kotlinx.serialization.ExperimentalSerializationApi::class)

package verification.serialization

import androidx.navigation3.runtime.NavKey
import com.google.samples.apps.nowinandroid.feature.bookmarks.api.navigation.BookmarksNavKey
import com.google.samples.apps.nowinandroid.feature.search.api.navigation.SearchNavKey
import kotlinx.serialization.KSerializer
import kotlinx.serialization.PolymorphicSerializer
import kotlinx.serialization.SerializationException
import kotlinx.serialization.descriptors.StructureKind
import kotlinx.serialization.json.Json
import kotlinx.serialization.modules.SerializersModule
import kotlinx.serialization.modules.polymorphic
import kotlinx.serialization.modules.subclass
import kotlinx.serialization.serializer

private fun observe(id: String, action: () -> Unit) {
    action()
    println("{\"id\":\"$id\",\"outcome\":\"pass\"}")
}

private fun rejects(action: () -> Unit) {
    var rejected = false
    try {
        action()
    } catch (_: SerializationException) {
        rejected = true
    } catch (_: IllegalArgumentException) {
        rejected = true
    }
    check(rejected)
}

private fun <T : Any> singleton(name: String, value: T, direct: KSerializer<T>, reified: KSerializer<T>) {
    observe("$name/direct_serializer") {
        check(direct === reified)
        check(direct.descriptor.serialName == value.javaClass.name)
        check(direct.descriptor.kind == StructureKind.OBJECT)
        check(direct.descriptor.elementsCount == 0)
        check(!direct.descriptor.isNullable)
    }
    observe("$name/json_roundtrip") {
        check(Json.encodeToString(direct, value) == "{}")
        check(Json.decodeFromString(direct, "{}") === value)
        check(Json.decodeFromString(direct, "{  }") === value)
        repeat(50) { check(Json.decodeFromString(direct, Json.encodeToString(reified, value)) === value) }
    }
    for ((id, input) in listOf("unknown_key" to "{\"extra\":1}", "malformed" to "{", "null" to "null", "array" to "[]", "primitive" to "1")) {
        observe("$name/reject_$id") { rejects { Json.decodeFromString(direct, input) } }
    }
    observe("$name/ignore_unknown_keys") {
        val tolerant = Json { ignoreUnknownKeys = true }
        check(tolerant.decodeFromString(direct, "{\"extra\":{\"nested\":[1,true,null]}}") === value)
    }
}

fun main() {
    singleton("bookmarks", BookmarksNavKey, BookmarksNavKey.serializer(), serializer<BookmarksNavKey>())
    singleton("search", SearchNavKey, SearchNavKey.serializer(), serializer<SearchNavKey>())
    val module = SerializersModule {
        polymorphic(NavKey::class) {
            subclass(BookmarksNavKey::class, BookmarksNavKey.serializer())
            subclass(SearchNavKey::class, SearchNavKey.serializer())
        }
    }
    val json = Json { serializersModule = module }
    val polymorphic = PolymorphicSerializer(NavKey::class)
    for ((name, key) in listOf("bookmarks" to BookmarksNavKey, "search" to SearchNavKey)) {
        observe("$name/polymorphic_roundtrip") {
            val expected = "{\"type\":\"${key.javaClass.name}\"}"
            check(json.encodeToString(polymorphic, key) == expected)
            check(json.decodeFromString(polymorphic, expected) === key)
        }
        observe("$name/polymorphic_unknown_key") {
            rejects { json.decodeFromString(polymorphic, "{\"type\":\"${key.javaClass.name}\",\"bad\":1}") }
        }
        observe("$name/polymorphic_missing_registration") {
            rejects { Json.encodeToString(polymorphic, key) }
        }
    }
    for ((name, input) in listOf("unknown_type" to "{\"type\":\"unknown.NavKey\"}", "missing_type" to "{}", "malformed" to "{\"type\":", "null" to "null")) {
        observe("polymorphic/reject_$name") { rejects { json.decodeFromString(polymorphic, input) } }
    }
}
