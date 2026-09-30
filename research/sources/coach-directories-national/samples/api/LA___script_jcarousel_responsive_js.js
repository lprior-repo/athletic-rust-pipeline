
(function($) {
    $(function() {
        var jcarousel = $('.jcarousel');

        jcarousel
            .on('jcarousel:reload jcarousel:create', function () {
                var carousel = $(this),
                    width = carousel.innerWidth();

                if (width >= 1000) {
                    width = width / 3;
                } else if (width >= 736) {
                    width = width / 2;
                } else if (width >= 424) {
                    width = width / 1;
                }

                carousel.jcarousel('items').css('width', Math.ceil(width) + 'px');
            })
            .jcarousel({
                wrap: 'circular'
            });

        $('.jcarousel-control-prev')
            .jcarouselControl({
                target: '-=1'
            });

        $('.jcarousel-control-next')
            .jcarouselControl({
                target: '+=1'
            });

        $('.jcarousel-pagination')
            .on('jcarouselpagination:active', 'a', function() {
                $(this).addClass('active');
            })
            .on('jcarouselpagination:inactive', 'a', function() {
                $(this).removeClass('active');
            })
            .on('click', function(e) {
                e.preventDefault();
            })
            .jcarouselPagination({
                perPage: 1,
                item: function(page) {
                    return '<a href="#' + page + '">' + page + '</a>';
                }
            });
    });
})(jQuery);


(function($) {
    $(function() {
        var jcarousel = $('#LogoTabList .jcarousel');

        jcarousel
            .on('jcarousel:reload jcarousel:create', function () {
                var carousel = $(this),
                    width = carousel.innerWidth();

                if (width >= 1000) {
                    width = width /4;
                }else if (width >=736) {
                    width = width /3;
                } else if (width >= 424) {
                    width = width / 2;
                }else if (width >= 250) {
                    width = width / 1;
                }
                carousel.jcarousel('items').css('width', Math.ceil(width) + 'px');
            })
            .jcarousel({
                wrap: 'circular'
            });

        $('#LogoTabList .jcarousel-control-prev')
            .jcarouselControl({
                target: '-=1'
            });

        $('#LogoTabList .jcarousel-control-next')
            .jcarouselControl({
                target: '+=1'
            });

        $('.jcarousel-pagination')
            .on('jcarouselpagination:active', 'a', function() {
                $(this).addClass('active');
            })
            .on('jcarouselpagination:inactive', 'a', function() {
                $(this).removeClass('active');
            })
            .on('click', function(e) {
                e.preventDefault();
            })
            .jcarouselPagination({
                perPage: 1,
                item: function(page) {
                    return '<a href="#' + page + '">' + page + '</a>';
                }
            });
    });
})(jQuery);


(function($) {
    $(function() {
        var jcarousel = $('#LogoList .jcarousel');

        jcarousel
            .on('jcarousel:reload jcarousel:create', function () {
                var carousel = $(this),
                    width = carousel.innerWidth();

                if (width >= 1000) {
                    width = width /5;
                }else if (width >=736) {
                    width = width /3;
                } else if (width >= 424) {
                    width = width / 2;
                }else if (width >= 250) {
                    width = width / 1;
                }
                carousel.jcarousel('items').css('width', Math.ceil(width) + 'px');
            })
            .jcarousel({
                wrap: 'circular'
            });

        $('#LogoList .jcarousel-control-prev')
            .jcarouselControl({
                target: '-=1'
            });

        $('LogoList .jcarousel-control-next')
            .jcarouselControl({
                target: '+=1'
            });

        $('.jcarousel-pagination')
            .on('jcarouselpagination:active', 'a', function() {
                $(this).addClass('active');
            })
            .on('jcarouselpagination:inactive', 'a', function() {
                $(this).removeClass('active');
            })
            .on('click', function(e) {
                e.preventDefault();
            })
            .jcarouselPagination({
                perPage: 1,
                item: function(page) {
                    return '<a href="#' + page + '">' + page + '</a>';
                }
            });
    });
})(jQuery);


(function($) {
    $(function() {
        var jcarousel = $('#GetConnectedList .jcarousel');

        jcarousel
            .on('jcarousel:reload jcarousel:create', function () {
                var carousel = $(this),
                    width = carousel.innerWidth();

                if (width >= 1000) {
                    width = width /4;
                }else if (width >=736) {
                    width = width /4;
                } else if (width >= 424) {
                    width = width / 2;
                }else if (width >= 250) {
                    width = width / 1;
                }
                carousel.jcarousel('items').css('width', Math.ceil(width) + 'px');
            })
            .jcarousel({
                wrap: 'circular'
            });

        $('#GetConnectedList .jcarousel-control-prev')
            .jcarouselControl({
                target: '-=1'
            });

        $('#GetConnectedList .jcarousel-control-next')
            .jcarouselControl({
                target: '+=1'
            });

        $('.jcarousel-pagination')
            .on('jcarouselpagination:active', 'a', function() {
                $(this).addClass('active');
            })
            .on('jcarouselpagination:inactive', 'a', function() {
                $(this).removeClass('active');
            })
            .on('click', function(e) {
                e.preventDefault();
            })
            .jcarouselPagination({
                perPage: 1,
                item: function(page) {
                    return '<a href="#' + page + '">' + page + '</a>';
                }
            });
    });
})(jQuery);


(function($) {
    $(function() {
        var jcarousel = $('#ExecutiveCommittee .jcarousel');

        jcarousel
            .on('jcarousel:reload jcarousel:create', function () {
                var carousel = $(this),
                    width = carousel.innerWidth();

                if (width >= 1000) {
                    width = width /5;
                }else if (width >=736) {
                    width = width /3;
                } else if (width >= 424) {
                    width = width / 2;
                }else if (width >= 250) {
                    width = width / 1;
                }
                carousel.jcarousel('items').css('width', Math.ceil(width) + 'px');
            })
            .jcarousel({
                wrap: 'circular'
            });

        $('#ExecutiveCommittee .jcarousel-control-prev')
            .jcarouselControl({
                target: '-=1'
            });

        $('#ExecutiveCommittee.jcarousel-control-next')
            .jcarouselControl({
                target: '+=1'
            });

        $('.jcarousel-pagination')
            .on('jcarouselpagination:active', 'a', function() {
                $(this).addClass('active');
            })
            .on('jcarouselpagination:inactive', 'a', function() {
                $(this).removeClass('active');
            })
            .on('click', function(e) {
                e.preventDefault();
            })
            .jcarouselPagination({
                perPage: 1,
                item: function(page) {
                    return '<a href="#' + page + '">' + page + '</a>';
                }
            });
    });
})(jQuery);


(function($) {
    $(function() {
        var jcarousel = $('#FtrGList .jcarousel');

        jcarousel
            .on('jcarousel:reload jcarousel:create', function () {
                var carousel = $(this),
                    width = carousel.innerWidth();

                if (width >= 1000) {
                    width = width /4;
                }else if (width >=736) {
                    width = width /3;
                } else if (width >= 424) {
                    width = width / 2;
                }else if (width >= 250) {
                    width = width / 1;
                }
                carousel.jcarousel('items').css('width', Math.ceil(width) + 'px');
            })
            .jcarousel({
                wrap: 'circular'
            });

        $('#FtrGList .jcarousel-control-prev')
            .jcarouselControl({
                target: '-=1'
            });

        $('#FtrGList .jcarousel-control-next')
            .jcarouselControl({
                target: '+=1'
            });

        $('.jcarousel-pagination')
            .on('jcarouselpagination:active', 'a', function() {
                $(this).addClass('active');
            })
            .on('jcarouselpagination:inactive', 'a', function() {
                $(this).removeClass('active');
            })
            .on('click', function(e) {
                e.preventDefault();
            })
            .jcarouselPagination({
                perPage: 1,
                item: function(page) {
                    return '<a href="#' + page + '">' + page + '</a>';
                }
            });
    });
})(jQuery);


(function($) {
    $(function() {
        var jcarousel = $('#FtrGalleryList .jcarousel');

        jcarousel
            .on('jcarousel:reload jcarousel:create', function () {
                var carousel = $(this),
                    width = carousel.innerWidth();

                if (width >= 1000) {
                    width = width /5;
                }else if (width >=736) {
                    width = width /3;
                } else if (width >= 424) {
                    width = width / 2;
                }else if (width >= 250) {
                    width = width / 1;
                }
                carousel.jcarousel('items').css('width', Math.ceil(width) + 'px');
            })
            .jcarousel({
                wrap: 'circular'
            });

        $('#FtrGalleryList .jcarousel-control-prev')
            .jcarouselControl({
                target: '-=1'
            });

        $('#FtrGalleryList .jcarousel-control-next')
            .jcarouselControl({
                target: '+=1'
            });

        $('.jcarousel-pagination')
            .on('jcarouselpagination:active', 'a', function() {
                $(this).addClass('active');
            })
            .on('jcarouselpagination:inactive', 'a', function() {
                $(this).removeClass('active');
            })
            .on('click', function(e) {
                e.preventDefault();
            })
            .jcarouselPagination({
                perPage: 1,
                item: function(page) {
                    return '<a href="#' + page + '">' + page + '</a>';
                }
            });
    });
})(jQuery);


(function($) {
    $(function() {
        var jcarousel = $('#LogoSlider .jcarousel');

        jcarousel
            .on('jcarousel:reload jcarousel:create', function () {
                var carousel = $(this),
                    width = carousel.innerWidth();

                if (width >= 1000) {
                    width = width /1;
                }else if (width >=736) {
                    width = width /1;
                } else if (width >= 424) {
                    width = width / 1;
                }else if (width >= 250) {
                    width = width / 1;
                }
                carousel.jcarousel('items').css('width', Math.ceil(width) + 'px');
            })
            .jcarousel({
                wrap: 'circular'
            });
            $('#LogoSlider .jcarousel')
                .jcarouselAutoscroll({
                interval: 4000,
                target: '+=1',
                autostart: true
            });
        $('#LogoSlider .jcarousel-control-prev')
            .jcarouselControl({
                target: '-=1'
            });

        $('#LogoSlider .jcarousel-control-next')
            .jcarouselControl({
                target: '+=1'
            });

        $('.jcarousel-pagination')
            .on('jcarouselpagination:active', 'a', function() {
                $(this).addClass('active');
            })
            .on('jcarouselpagination:inactive', 'a', function() {
                $(this).removeClass('active');
            })
            .on('click', function(e) {
                e.preventDefault();
            })
            .jcarouselPagination({
                perPage: 1,
                item: function(page) {
                    return '<a href="#' + page + '">' + page + '</a>';
                }
            });
    });
})(jQuery);
(function($) {
    $(function() {
        var jcarousel = $('#IdxProjectGallerySlider .jcarousel');

        jcarousel
            .on('jcarousel:reload jcarousel:create', function () {
                var carousel = $(this),
                    width = carousel.innerWidth();

                if (width >= 1000) {
                    width = width /1;
                }else if (width >=736) {
                    width = width /1;
                } else if (width >= 424) {
                    width = width / 1;
                }else if (width >= 250) {
                    width = width / 1;
                }
                carousel.jcarousel('items').css('width', Math.ceil(width) + 'px');
            })
            .jcarousel({
                wrap: 'circular'
            });
            $('#IdxProjectGallerySlider .jcarousel')
                .jcarouselAutoscroll({
                interval: 4000,
                target: '+=1',
                autostart: true
            });
        $('#IdxProjectGallerySlider .jcarousel-control-prev')
            .jcarouselControl({
                target: '-=1'
            });

        $('#IdxProjectGallerySlider .jcarousel-control-next')
            .jcarouselControl({
                target: '+=1'
            });

        $('.jcarousel-pagination')
            .on('jcarouselpagination:active', 'a', function() {
                $(this).addClass('active');
            })
            .on('jcarouselpagination:inactive', 'a', function() {
                $(this).removeClass('active');
            })
            .on('click', function(e) {
                e.preventDefault();
            })
            .jcarouselPagination({
                perPage: 1,
                item: function(page) {
                    return '<a href="#' + page + '">' + page + '</a>';
                }
            });
    });
})(jQuery);


