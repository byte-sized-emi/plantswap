package app.plantswap.frontend

interface Platform {
    val name: String
}

expect fun getPlatform(): Platform
