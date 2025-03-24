function Comic(image, title) {
    this.image = url;
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
            cached[id] = p;
            p.then(comic => prefetchComic(comic.image));
            return p;
        }
    }

    function prefetchComic(comic) {
        $("#prefetch").append("<img src=\"" + comic.image + "\">");
    }

    function startCaching(number) {
        getComic(number - 1);
        getComic(number + 1);
        nextRand = Math.floor(Math.random() * maxComics) + 1;
        getComic(nextRand);
    }

    function showComic(number, fromBackButton) {
        if(fromBackButton == false)
            history.pushState(number, "#" + number.toString(), "?comic=" + number);
        // This removes the title box if it's there
        $("#titleBox").click();
        getComic(number).then(comic => {
            $("#imageHolder").html("<img src=\"" + comic.image + "\">");
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
    $("#left").click(function() {
        if (comicNum != undefined) {
            comicNum--;
            showComic(comicNum, false);
        }
    });
    $("#right").click(function() {
        if (comicNum != undefined) {
            comicNum++;
            showComic(comicNum, false);
        }
    });
    $("#random").click(async function() {
        let maxComicId = await maxComics;
        if (nextRand == undefined) {
            nextRand = Math.floor(Math.random() * maxComics) + 1;
        }
        comicNum = nextRand;
        showComic(comicNum, false);
        nextRand = undefined;
        startCaching(comicNum);
    });
    $("#showTitle").click(async function() {
        var $titleBox = $("#titleBox");
        if ($titleBox.css("display") === "none") {
            $titleBox.html((await comics[comicNum]).title);
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
        comicNum = maxComics.then(id => {
            showComic(id, false);
        });
    }
});
