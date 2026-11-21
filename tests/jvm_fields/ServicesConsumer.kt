package servicesconsumer
import java.lang.reflect.Proxy
import yuku.alkitab.base.services.*
fun main() {
    fun <T> service(type: Class<T>): T = type.cast(Proxy.newProxyInstance(type.classLoader, arrayOf(type)) { _, method, _ -> throw AssertionError(method) })
    val storage = service(StorageProvider::class.java)
    val versions = service(VersionManager::class.java)
    val dimensions = service(UiDimensionsProvider::class.java)
    val value = AppServices(storage, versions, dimensions)
    check(value.storage === storage && value.versions === versions && value.uiDimensions === dimensions)
    println("OK")
}
