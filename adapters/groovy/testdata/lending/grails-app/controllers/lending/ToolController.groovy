package lending

class ToolController {

    def catalogService

    def show(long id) {
        [tool: catalogService.toolById(id)]
    }
}
