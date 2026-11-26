package modules

import com.sun.net.httpserver.HttpServer
import com.sun.net.httpserver.HttpHandler
import java.net.InetSocketAddress
import java.net.http.HttpClient
import java.sql.DriverManager
import javax.xml.parsers.DocumentBuilderFactory
import org.w3c.dom.Document
import sun.misc.Unsafe
import jdk.incubator.vector.IntVector

fun http(): HttpServer = HttpServer.create(InetSocketAddress("127.0.0.1", 0), 0)
fun handler(): HttpHandler? = null
fun xml(): Document = DocumentBuilderFactory.newInstance().newDocumentBuilder().newDocument()
fun driverVersion(): Int = DriverManager.getLoginTimeout()

fun version(): HttpClient.Version = HttpClient.Version.HTTP_1_1

fun unsafe(): Unsafe? = null
