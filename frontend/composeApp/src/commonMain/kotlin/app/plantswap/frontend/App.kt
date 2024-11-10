package app.plantswap.frontend

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material.Button
import androidx.compose.material.MaterialTheme
import androidx.compose.material.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.navigation.NavController
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import org.jetbrains.compose.ui.tooling.preview.Preview

enum class PlantSwapScreens(val title: String) {
    Discover(title = "Discover"),
    CreateListing(title = "Create Listing"),
    About(title = "About"),
    SpecificListing(title = "Listing")
}

@Composable
@Preview
fun App() {
    val navController = rememberNavController()

    MaterialTheme {
        Column(Modifier.fillMaxWidth()) {
            NavHost(
                navController = navController,
                startDestination = PlantSwapScreens.CreateListing.name
            ) {
                composable(route = PlantSwapScreens.About.name) {
                    AboutScreen()
                }
                composable(route = PlantSwapScreens.Discover.name) {
                    DiscoverScreen(
                        navController = navController
                    )
                }
                composable(route = PlantSwapScreens.CreateListing.name) {
                    CreateListingScreen(navController = navController)
                }
                composable(route = PlantSwapScreens.SpecificListing.name + "/{listingId}",
                    arguments = listOf(navArgument("listingId") { type = NavType.LongType })
                )
                {
                    val listingId = it.arguments?.getLong("listingId")
                    Text("Listing with id $listingId")
                }
            }
        }
    }
}

@Composable
fun DiscoverScreen(navController: NavController) {
    Button(onClick = { navController.navigate(PlantSwapScreens.SpecificListing.name + "/10") }) {
        Text("FriendsListScreen")
    }
}

@Composable
fun AboutScreen() {
    Text("ProfileScreen")
}
