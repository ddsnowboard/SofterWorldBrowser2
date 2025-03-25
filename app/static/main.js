function Comic(image, title) {
    this.image = image;
    this.title = title;
}
let comics = {}
$(document).ready(function() {
    function comicUrl(id) {
        return "getComic/" + id.toString()
    }

    function getComic(id) {
        let cached = comics[id];
        if(cached !== undefined) {
            return cached
        } else {
            let p = fetch(comicUrl(id))
                .then(res => res.json())
                .then(jsonObj => new Comic(jsonObj["image"], jsonObj["title"]));
            comics[id] = p;
            return p;
        }
    }

    function startCaching(number) {
        Promise.all([
            getComic(number - 1),
            getComic(number + 1),
            (async () => {
                nextRand = Math.floor(Math.random() * (await maxComics)) + 1;
                await getComic(nextRand);
            })()]);
    }

    function showComic(number, fromBackButton) {
        if(fromBackButton == false)
            history.pushState(number, "#" + number.toString(), "?comic=" + number);
        // This removes the title box if it's there
        $("#titleBox").click();
        getComic(number).then(comic => {
            $("#imageHolder").html("<img src=\"data:image/png;base64, " + comic.image + "\">");
        });
        $("#number").html(number);
        startCaching(number);
    }
    window.onpopstate = function(state)
    {
        showComic(state.state, true);
    };
    const maxComics = fetch("/maxComicId").then(res => res.text()).then(txt => parseInt(txt));
    var comicNum;
    var nextRand;
    $("#left").click(async function() {
        if (comicNum != undefined) {
            comicNum = (await comicNum) - 1;
            showComic(comicNum, false);
        }
    });
    $("#right").click(async function() {
        if (comicNum != undefined) {
            comicNum = (await comicNum) + 1;
            showComic(comicNum, false);
        }
    });
    $("#random").click(async function() {
        let maxComicId = await maxComics;
        if (nextRand === undefined) {
            nextRand = Math.floor(Math.random() * maxComicId) + 1;
            console.log("NEW RNG");
        }
        comicNum = nextRand;
        showComic(comicNum, false);
        startCaching(comicNum);
    });
    $("#showTitle").click(async function() {
        var $titleBox = $("#titleBox");
        if ($titleBox.css("display") === "none") {
            $titleBox.html((await getComic(comicNum)).title);
            $titleBox.css("display", "block");
        } else
            $titleBox.click();
    });
    $(document).keydown(function(event) {
        var ESCAPE = 27;
        var Z = 90;
        var C = 67;
        var S = 83;
        var X = 88;
        switch(event.which) {
            case ESCAPE:
                $("#titleBox").click();
                break;
            case Z:
                $("#left").click();
                break;
            case C:
                $("#right").click();
                break;
            case S:
                $("#random").click();
                break;
            case X:
                $("#showTitle").click();
                break;
        }
    });
    var lastTouchStartingX = 0;
    var lastTouchStartingY = 0;
    var ignoringLift = true;
    document.getElementById("imageHolder").addEventListener("touchstart", function(event) {
        if(event.touches.length > 1)
        {
            ignoringLift = true;
            return;
        }
        ignoringLift = false;
        event.preventDefault();
        lastTouchStartingX = event.changedTouches[0].screenX;
        lastTouchStartingY = event.changedTouches[0].screenY;
    });

    document.getElementById("imageHolder").addEventListener("touchend", function(event) {
        if(ignoringLift)
        {
            ignoringLift = true;
            return;
        }
        var MINIMUM_DELTA = 50;
        event.preventDefault();
        var newX = event.changedTouches[0].screenX;
        var newY = event.changedTouches[0].screenY;
        if (Math.abs(newX - lastTouchStartingX) >= MINIMUM_DELTA) {
            if (newX > lastTouchStartingX)
                $("#left").click();
            else
                $("#right").click();
        }
        else if(Math.abs(newY - lastTouchStartingY) >= MINIMUM_DELTA)
            $("#random").click();
        else
            $("#showTitle").click();
    });
    $("#titleBox").click(function() {
        if ($(this).css("display") === "block")
            $("#titleBox").css("display", "none");
    });

    // This handles the possibility of the browser recovering from a 
    // crash or someone clicking on a link to get a comic and having
    // a comic number in the URL from a session that we don't have
    // on the history stack. This is only run when you first load
    // the site.
    var queryString = new URLSearchParams(window.location.search);
    var queryHasComic = queryString.has("comic");
    if(queryHasComic) {
        comicNum = parseInt(queryString.get("comic"));
        showComic(comicNum, false);
    } else {
        comicNum = maxComics
        comicNum.then(id => {
            console.log("LOOKING TO GET ID " + id);
            showComic(id, false);
        });
    }
});
